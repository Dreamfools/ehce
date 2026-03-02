use crate::common::textures::{SpriteAsset, TextureCache};
use crate::loading::staggerer::{Staggerer, StaggererImpl};
use assets_manager::{
    Asset, AssetCache, BoxedError, FileAsset, Handle, RecursiveDirectory, SharedBytes,
};
use facet::{Facet, Shape};
use itertools::Itertools as _;
use miette::{IntoDiagnostic as _, Report};
use registry::registry::entry_seen;
use registry::registry::id::RawId;
use registry::registry::reflect_registry::{BuildReflectRegistry, ReflectRegistry};
use std::borrow::Cow;
use tracing::{debug, info, trace, warn};
use crate::common::textures::texture::TextureMetadata;
use crate::parsing::load_toml;

mod staggerer;

struct ImageBytes(SharedBytes);

impl From<SharedBytes> for ImageBytes {
    fn from(bytes: SharedBytes) -> ImageBytes {
        ImageBytes(bytes)
    }
}

impl FileAsset for ImageBytes {
    const EXTENSIONS: &'static [&'static str] = &["png", "jpg", "jpeg"];

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        Ok(ImageBytes(SharedBytes::from(bytes)))
    }
}

impl FileAsset for TextureMetadata {
    const EXTENSION: &'static str = "meta";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        load_toml(&bytes)
    }
}

// impl Asset for RegistryItemSerialized {
//     const EXTENSION: &'static str = "json";
//     type Loader = loader::JsonLoader;
// }

// type ItemsHandle = Handle<RecursiveDirectory<RegistryItemSerialized>>;
type ImagesHandle = Handle<RecursiveDirectory<ImageBytes>>;
type TextureMetadataHandle = Handle<RecursiveDirectory<TextureMetadata>>;

#[derive(Debug)]
pub struct LoadedMod<'a, A: Model> {
    registry: ReflectRegistry,
    cache: &'a AssetCache,
    items: &'a Handle<RecursiveDirectory<A>>,
    images: &'a ImagesHandle,
    images_meta: &'a TextureMetadataHandle,

    hot_reload_stagger: StaggererImpl,
    want_full_reload: bool,
}

pub trait Model: FileAsset + Facet<'static> + Clone {
    fn required_singletons() -> Vec<&'static Shape>;
}

impl<'a, A: Model> LoadedMod<'a, A> {
    #[must_use]
    pub fn registry(&self) -> &ReflectRegistry {
        &self.registry
    }

    /// Loads a mod given the asset cache
    ///
    /// Errors can be safely handed, and should not affect any global state
    pub fn load_mod(cache: &'a AssetCache) -> Result<Self, Report> {
        Self::load_mod_inner(cache)
    }

    /// Performs hot-reload, updating the mod accordingly, returning `true` if
    /// data files were reloaded (not images or other assets)
    ///
    /// Hot reload is guaranteed to not alter existing loaded item IDs
    ///
    /// Errors can be safely handed, and should not affect any global state
    ///
    /// Some changes can't be safely hot-reloaded, call
    /// [LoadedMod::want_full_reload] to check if a full reload is required
    pub fn hot_reload(&mut self) -> Result<bool, Report> {
        if !self.cache.is_hot_reloaded() {
            return Ok(false);
        }

        self.update_images::<false>().into_diagnostic()?;

        if self.want_files_hot_reload().into_diagnostic()? {
            trace!("[Model]: File reload detected, queueing hot reload");
            self.hot_reload_stagger.trigger();
        }

        if self.hot_reload_stagger.activated() {
            info!("[Model]: Hot reloading data files");
            let loaded = Self::load_mod_inner(self.cache)?;

            if loaded.registry.ids_equal(&self.registry) {
                mark_texture_meta_dirty(&self.registry, &loaded.registry);
                self.registry = loaded.registry;
                self.items = loaded.items;
                self.images = loaded.images;
                self.want_full_reload = false;
                Ok(true)
            } else {
                warn!("[Model]: Full reload is required");
                self.want_full_reload = true;
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Indicates that a full reload is required, because some changes are not
    /// supported by hot-reload
    ///
    /// This should be called after [LoadedMod::hot_reload]
    #[must_use]
    pub fn want_full_reload(&self) -> bool {
        self.want_full_reload
    }
}

impl<'a, A: Model> LoadedMod<'a, A> {
    fn load_mod_inner(cache: &'a AssetCache) -> Result<Self, Report> {
        let image_handles = cache.load_rec_dir::<ImageBytes>("").into_diagnostic()?;
        let image_meta_handles = cache.load_rec_dir::<TextureMetadata>("").into_diagnostic()?;
        let item_handles = cache.load_rec_dir::<A>("").into_diagnostic()?;

        let images = load_items(cache, image_handles).into_diagnostic()?;
        let image_metadata = load_items(cache, image_meta_handles).into_diagnostic()?;
        let items = load_items(cache, item_handles).into_diagnostic()?;

        let mut reg = BuildReflectRegistry::default();
        reg.expect_singletons(A::required_singletons());

        for data in items {
            let asset = &*data.read();

            let path = registry::path::FieldPath::new(&format!("[{}]", data.id()));
            registry::traverse::traverse(asset, &path, &mut reg)?;
        }

        for data in image_metadata {
            let asset = &*data.read();
            let field_path = registry::path::FieldPath::new(&format!("[{}]", data.id()));

            registry::traverse::traverse(asset, &field_path, &mut reg)?;
            entry_seen(&mut reg, &field_path, RawId::new(data.id().as_str()), asset)?;
        }

        let mut sprites = Vec::with_capacity(images.len());
        for data in images {
            let field_path = registry::path::FieldPath::new(&format!("[{}]", data.id()));
            let sprite_id = SpriteAsset(data.id().to_string().into());
            entry_seen(
                &mut reg,
                &field_path,
                RawId::new(data.id().as_str()),
                &sprite_id,
            )?;

            sprites.push(data);
        }

        let registry = reg.build().into_diagnostic()?;

        let mod_data = Self {
            registry,
            cache,
            items: item_handles,
            images: image_handles,
            images_meta: image_meta_handles,
            hot_reload_stagger: Staggerer::new(0.150, 1.0),
            want_full_reload: false,
        };

        mod_data.update_images::<true>().into_diagnostic()?;

        Ok(mod_data)
    }

    /// Updates all images. If error happens, no changes are performed
    ///
    /// If `FORCE` is true, all textures would be updated, otherwise only hot
    /// reloaded textures would
    fn update_images<const FORCE: bool>(&self) -> Result<(), assets_manager::Error> {
        let mut changes = vec![];
        for data in self.images.read().iter(self.cache) {
            let handle = data?;
            if FORCE || handle.reloaded_global() {
                if !FORCE {
                    debug!("[Model]: Updating image `{}`", handle.id());
                }
                changes.push(handle);
            }
        }

        TextureCache::with(|cache| {
            for image in changes {
                let bytes = image.read().0.clone();
                cache.add_texture(image.id().to_string(), bytes);
            }
        });

        Ok(())
    }

    fn want_files_hot_reload(&self) -> Result<bool, assets_manager::Error> {
        let mut any_reloaded = false;
        for data in self.items.read().iter(self.cache) {
            let handle = data?;
            if handle.reloaded_global() {
                any_reloaded = true;
            }
        }
        for data in self.images_meta.read().iter(self.cache) {
            let handle = data?;
            if handle.reloaded_global() {
                any_reloaded = true;
            }
        }
        Ok(any_reloaded)
    }
}
fn load_items<'a, T: Asset>(
    cache: &'a AssetCache,
    input: &'a Handle<RecursiveDirectory<T>>,
) -> Result<Vec<&'a Handle<T>>, assets_manager::Error> {
    let data: Vec<_> = input
        .read()
        .iter(cache)
        .try_collect()?;

    Ok(data)
}

fn mark_texture_meta_dirty(old: &ReflectRegistry, new: &ReflectRegistry) {
    TextureCache::with(|cache| {
        for x in old.item_differences::<TextureMetadata>(new) {
            cache.mark_metadata_dirty(x.raw().as_str());
        }
    });
}