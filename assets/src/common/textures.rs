use assets_manager::SharedBytes;
use facet::Facet;
use macroquad::math::UVec2;
use parking_lot::{RwLock, RwLockWriteGuard};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::fmt::{Debug, Formatter};
use std::ops::Deref as _;
use std::sync::OnceLock;
use utils::map::HashMap;
use yakui::paint::{TextureFormat};
use yakui::ManagedTextureId;
use registry::registry::id::{IdRef, RawId};
use registry::registry::reflect_registry::ReflectRegistry;
use crate::common::textures::texture::{TextureMetadata};

pub mod sprite;
pub mod filter_mode;
pub mod texture;

type TextureBytes = SharedBytes;

#[derive(Debug)]
struct ImageHandle {
    bytes: TextureBytes,
    macroquad: Option<macroquad::texture::Texture2D>,
    yakui: Option<yakui::TextureId>,
    macroquad_meta_dirty: bool,
}

impl ImageHandle {
    pub fn new(bytes: TextureBytes) -> Self {
        Self {
            bytes,
            macroquad: None,
            yakui: None,
            macroquad_meta_dirty: true,
        }
    }
}

static HANDLES: OnceLock<RwLock<HashMap<String, ImageHandle>>> = OnceLock::new();

fn texture_handles() -> &'static RwLock<HashMap<String, ImageHandle>> {
    HANDLES.get_or_init(|| RwLock::new(HashMap::default()))
}

#[derive(Debug)]
pub struct TextureCache(RwLockWriteGuard<'static, HashMap<String, ImageHandle>>);

impl TextureCache {
    // #[must_use]
    // fn open() -> Self {
    //     Self(texture_handles().write())
    // }

    pub fn with(cb: impl FnOnce(&mut Self)) {
        let mut cache = Self(texture_handles().write());
        cb(&mut cache);
    }

    pub fn add_texture(&mut self, k: String, bytes: TextureBytes) {
        match self.0.entry(k) {
            Entry::Occupied(mut entry) => {
                // Bytes are the same, so skip replacing
                if entry.get().bytes == bytes {
                    return;
                }
                let old = entry.insert(ImageHandle::new(bytes));

                // Macroquad does texture GC by itself, so only worry about yakui
                if let Some(_id) = old.yakui {
                    // TODO: delete old yakui texture
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(ImageHandle::new(bytes));
            }
        }
    }
    
    pub fn mark_metadata_dirty(&mut self, k: &str) {
        if let Some(handle) = self.0.get_mut(k) {
            handle.macroquad_meta_dirty = true;
            handle.yakui = None; // Yakui texure metadata can only be set on creation
            // TODO: delete old yakui texture
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, Facet)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SpriteAsset(pub(crate) Cow<'static, str>);
pub type SpriteId = IdRef<SpriteAsset>;

impl From<SpriteId> for SpriteAsset {
    fn from(value: SpriteId) -> Self {
        Self(value.raw().as_str().into())
    }
}
impl From<&SpriteId> for SpriteAsset {
    fn from(value: &SpriteId) -> Self {
        Self(value.raw().as_str().into())
    }
}

impl SpriteAsset {
    #[allow(clippy::significant_drop_tightening)]
    #[must_use]
    pub fn texture2d(&self, registry: &ReflectRegistry) -> macroquad::texture::Texture2D {
        let mut textures = texture_handles().write();


        let handle = textures.get_mut(self.0.as_ref()).unwrap();
        let texture2d = handle
            .macroquad
            .get_or_insert_with(|| {

                
                // tx.set_filter(meta.filter_mode.to_macroquad());
                macroquad::texture::Texture2D::from_file_with_format(handle.bytes.as_ref(), None)
            })
            .weak_clone();
        
        if handle.macroquad_meta_dirty {
            let id = RawId::new(self.0.deref());
            let meta = registry.get(IdRef::<TextureMetadata>::new(id)).cloned().unwrap_or_default();
            handle.macroquad_meta_dirty = false;

            texture2d.set_filter(meta.filter_mode.to_macroquad());
        }
        texture2d
    }

    #[allow(clippy::significant_drop_tightening)]
    #[must_use]
    pub fn yakui_texture(&self, registry: &ReflectRegistry) -> yakui::TextureId {
        let mut textures = texture_handles().write();

        let handle = textures.get_mut(self.0.as_ref()).unwrap();
        *handle.yakui.get_or_insert_with(|| {
            let id = RawId::new(self.0.deref());
            let meta = registry.get(IdRef::<TextureMetadata>::new(id)).cloned().unwrap_or_default();

            let image = image::load_from_memory(handle.bytes.as_ref())
                .unwrap()
                .into_rgba8();
            let size = UVec2::new(image.width(), image.height());

            let mut texture =
                yakui::paint::Texture::new(TextureFormat::Rgba8Srgb, yakui::UVec2::from(size.to_array()), image.into_raw());
            texture.mag_filter = meta.filter_mode.to_yakui();
            let mut texture_id = None::<ManagedTextureId>;
            yakui_macroquad::cfg(|yak| texture_id = Some(yak.add_texture(texture)));
            yakui::TextureId::Managed(texture_id.expect("texture id was set"))
        })
    }
}

impl Debug for SpriteAsset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}
