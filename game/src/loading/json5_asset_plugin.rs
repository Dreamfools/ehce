use std::marker::PhantomData;
use bevy::app::{App, Plugin};
use bevy::asset::{Asset, AssetApp, AssetLoader, LoadContext};
use bevy::asset::io::Reader;
use bevy::reflect::TypePath;
use thiserror::Error;

pub struct Json5AssetPlugin<A> {
    extensions: Vec<&'static str>,
    _marker: PhantomData<A>,
}

impl<A> Plugin for Json5AssetPlugin<A>
where
        for<'de> A: serde::Deserialize<'de> + Asset,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<A>()
            .register_asset_loader(Json5AssetLoader::<A> {
                extensions: self.extensions.clone(),
                _marker: PhantomData,
            });
    }
}

impl<A> Json5AssetPlugin<A>
where
        for<'de> A: serde::Deserialize<'de> + Asset,
{
    /// Create a new plugin that will load assets from files with the given extensions.
    pub fn new(extensions: &[&'static str]) -> Self {
        Self {
            extensions: extensions.to_owned(),
            _marker: PhantomData,
        }
    }
}

/// Loads your asset type `A` from json files
#[derive(TypePath)]
pub struct Json5AssetLoader<A> {
    extensions: Vec<&'static str>,
    _marker: PhantomData<A>,
}

/// Possible errors that can be produced by [`Json5AssetLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Json5LoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    /// A [JSON Error](serde_json::error::Error)
    #[error("Could not parse the JSON: {0}")]
    Json5Error(#[from] serde_json5::Error),
}

impl<A> AssetLoader for Json5AssetLoader<A>
where
        for<'de> A: serde::Deserialize<'de> + Asset,
{
    type Asset = A;
    type Settings = ();
    type Error = Json5LoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let asset = serde_json5::from_slice::<A>(&bytes)?;
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &self.extensions
    }
}