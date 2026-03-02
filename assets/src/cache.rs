use crate::loading::{LoadedMod, Model};
use assets_manager::{source, AssetCache};
use miette::{IntoDiagnostic as _, Report};
use ouroboros::self_referencing;

pub fn load_assets_cache<A: Model>(
    path: impl Into<String>,
) -> miette::Result<AssetsHolder<A>> {
    #[cfg(not(target_arch = "wasm32"))]
    let source = source::FileSystem::new(path.into()).into_diagnostic()?;

    #[cfg(target_arch = "wasm32")]
    let source = source::Embedded::from(source::embed!("game/mod"));

    let cache = AssetCache::with_source(source);

    let holder = AssetsHolder::try_new(cache, |cache| LoadedMod::<A>::load_mod(cache))?;

    Ok(holder)
}

#[self_referencing]
pub struct AssetsHolder<A: Model> {
    cache: AssetCache,

    #[borrows(cache)]
    #[covariant]
    mod_data: LoadedMod<'this, A>,
}

impl<A: Model> AssetsHolder<A> {
    pub fn hot_reload(&mut self) -> Result<bool, Report> {
        AssetsHolder::with_mod_data_mut(self, |cache| LoadedMod::hot_reload(cache))
    }

    #[must_use]
    pub fn mod_data(&self) -> &LoadedMod<'_, A> {
        self.borrow_mod_data()
    }

    #[must_use]
    pub fn asset_cache(&self) -> &AssetCache {
        self.borrow_cache()
    }
}
