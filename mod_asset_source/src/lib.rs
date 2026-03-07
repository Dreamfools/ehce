use bevy_asset::io::{AssetReader, AssetReaderError, PathStream, Reader};
use bevy_tasks::futures_lite::StreamExt as _;
use std::collections::hash_map::Entry;
use std::fmt::Display;
use std::ops::Deref as _;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::{env, io, path};
use utils::map::HashMap;

use tracing::{info, warn};

mod read_dir;
mod read_file;

pub static MODS_FOLDER: LazyLock<&'static str> = LazyLock::new(|| {
    if let Ok(mods_dir) = env::var("EHCE_MODS_DIR") {
        return Box::leak(mods_dir.into_boxed_str());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let base_path = bevy_asset::io::file::FileAssetReader::get_base_path();
    #[cfg(target_arch = "wasm32")]
    let base_path = PathBuf::from(".");
    let mods_path = base_path.join("mods");
    Box::leak(
        mods_path
            .to_str()
            .unwrap_or_else(|| panic!("Base path is not valid UTF-8"))
            .to_string()
            .into_boxed_str(),
    )
});

pub struct ModsAssetReader {
    systems: parking_lot::Mutex<Vec<Arc<ModFs>>>,
    mods: parking_lot::Mutex<Option<HashMap<String, Arc<ModFs>>>>,
}

impl Default for ModsAssetReader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModsAssetReader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            systems: parking_lot::Mutex::new(Vec::new()),
            mods: parking_lot::Mutex::new(None),
        }
    }

    /// Refreshes the list of mods. This should be called when trying to reload active mods
    pub fn refresh_mods(&self) {
        self.mark_dirty();
    }

    /// Adds an embedded directory to the asset reader. In debug builds, this
    /// will add a filesystem directory instead, allowing for changing of assets
    /// without recompilation
    pub fn add_embedded(&self, name: String, dir: EmbeddedOnRelease) {
        match dir {
            EmbeddedOnRelease::Embedded { dir } => {
                self.systems.lock().push(Arc::new(ModFs {
                    driver: Arc::new(FsDriver::Embedded { dir }),
                    ignore: Arc::new(Default::default()),
                    name,
                }));
            }
            EmbeddedOnRelease::FileSystem { root } => {
                #[cfg(target_arch = "wasm32")]
                panic!("Embedded drivers must be in the embedded format on wasm, but {} is a filesystem driver",name);
                self.add_filesystem(root);
            }
        }

        self.mark_dirty();
    }

    /// Adds a filesystem directory to the asset reader
    pub fn add_filesystem(&self, root: PathBuf) {
        if cfg!(target_arch = "wasm32") {
            warn!(root=%root.display(), "Filesystem mods are not supported on wasm, skipping");
            return;
        }

        let abs_path = match path::absolute(&root) {
            Ok(path) => path,
            Err(err) => {
                warn!(path = %root.display(), err = %err, "Mods folder path is invalid, skipping");
                return;
            }
        };
        let root = match abs_path.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                warn!(path = %abs_path.display(), err = %err, "Mods folder path is invalid or does not exist, skipping");
                return;
            }
        };

        self.systems.lock().push(Arc::new(ModFs {
            name: root.to_string_lossy().to_string(),
            driver: Arc::new(FsDriver::FileSystem { root }),
            ignore: Arc::new(Default::default()),
        }));

        self.mark_dirty();
    }

    #[allow(clippy::significant_drop_tightening)] // false positive due to as_ref()
    pub async fn list_mod_names(&self) -> Result<Vec<String>, AssetReaderError> {
        {
            let lock = self.mods.lock();
            if let Some(mods) = &*lock {
                return Ok(mods.keys().cloned().collect());
            }
        }
        self.update_mods().await?;
        let lock = self.mods.lock();
        let mods = lock
            .as_ref()
            .expect("Mods should be initialized after update_mods");
        Ok(mods.keys().cloned().collect())
    }

    fn mark_dirty(&self) {
        *self.mods.lock() = None;
    }

    #[allow(clippy::significant_drop_tightening)] // false positive due to as_ref()
    async fn get_mod(&self, name: &str) -> Result<Arc<ModFs>, AssetReaderError> {
        {
            let mods = self.mods.lock();
            if let Some(mods) = &*mods {
                return mods
                    .get(name)
                    .cloned()
                    .ok_or_else(|| custom_error(format!("Mod {} not found", name)));
            }
        }
        self.update_mods().await?;

        let mods = self.mods.lock();
        let mods = mods
            .as_ref()
            .expect("Mods should be initialized after update_mods");
        mods.get(name)
            .cloned()
            .ok_or_else(|| custom_error(format!("Mod {} not found", name)))
    }

    async fn update_mods(&self) -> Result<(), AssetReaderError> {
        let mut mods = HashMap::<String, Arc<ModFs>>::default();
        let active_systems = self.systems.lock().clone();
        for system in active_systems {
            info!("Listing mods in system {}", system);
            for fs in system.list_mods().await? {
                match mods.entry(fs.name.clone()) {
                    Entry::Vacant(e) => {
                        e.insert(Arc::new(fs));
                    }
                    Entry::Occupied(existing) => {
                        return Err(custom_error(format!(
                            "Mod is defined in multiple sources: {} is in both {} and {}",
                            existing.key(),
                            existing.get(),
                            system
                        )));
                    }
                }
            }
        }
        let mut lock = self.mods.lock();
        *lock = Some(mods);
        drop(lock);
        Ok(())
    }
}

fn custom_error(text: impl Into<String>) -> AssetReaderError {
    io::Error::other(text.into()).into()
}

fn utf8_file_name(path: &Path) -> Result<String, AssetReaderError> {
    Ok(path
        .file_name()
        .ok_or_else(|| custom_error(format!("Path {} has no file name", path.display())))?
        .to_str()
        .ok_or_else(|| {
            custom_error(format!(
                "File name of {} is not valid UTF-8",
                path.display()
            ))
        })?
        .to_owned())
}

fn mod_name_from_path(path: &Path) -> Result<String, AssetReaderError> {
    let Some(first) = path.components().next() else {
        return Err(custom_error(format!(
            "Path {} has no components",
            path.display()
        )));
    };
    match first {
        Component::Prefix(_) | Component::RootDir => Err(custom_error(format!(
            "Absolute paths are not allowed: {}",
            path.display()
        ))),
        Component::CurDir => Err(custom_error(format!(
            "Path cannot start with .: {}",
            path.display()
        ))),
        Component::ParentDir => Err(custom_error(format!(
            "Path cannot start with ..: {}",
            path.display()
        ))),
        Component::Normal(str) => {
            if str.is_empty() {
                return Err(custom_error(format!(
                    "Path cannot start with an empty component: {}",
                    path.display()
                )));
            }
            Ok(str
                .to_str()
                .ok_or_else(|| {
                    custom_error(format!(
                        "First component of {} is not valid UTF-8",
                        path.display()
                    ))
                })?
                .to_owned())
        }
    }
}

impl AssetReader for ModsAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader, AssetReaderError> {
        info!(path = %path.display(), "Reading asset from mods");
        let mod_name = mod_name_from_path(path)?;
        let fs = self.get_mod(&mod_name).await?;
        fs.read_file(path).await
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader, AssetReaderError> {
        info!(path = %path.display(), "Reading asset meta from mods");
        let mod_name = mod_name_from_path(path)?;
        let fs = self.get_mod(&mod_name).await?;
        fs.read_file(&get_meta_path(path)).await
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        info!(path = %path.display(), "Reading directory from mods");
        let mod_name = mod_name_from_path(path)?;
        let fs = self.get_mod(&mod_name).await?;
        Ok(Box::new(fs.read_directory(path).await?))
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        info!(path = %path.display(), "Checking if path is a directory in mods");
        let mod_name = mod_name_from_path(path)?;
        let fs = self.get_mod(&mod_name).await?;
        fs.is_directory(path).await
    }
}

impl AssetReader for &'static ModsAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<impl Reader, AssetReaderError>> {
        (**self).read(path)
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<impl Reader, AssetReaderError>> {
        (**self).read_meta(path)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<Box<PathStream>, AssetReaderError>> {
        (**self).read_directory(path)
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<bool, AssetReaderError>> {
        (**self).is_directory(path)
    }
}

pub enum EmbeddedOnRelease {
    Embedded { dir: include_dir::Dir<'static> },
    FileSystem { root: PathBuf },
}

#[macro_export]
macro_rules! embedded_assets {
    ($debug_root:literal, $($tts:tt)*) => {{
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        let fs = $crate::EmbeddedOnRelease::FileSystem {
            root: std::path::PathBuf::from($debug_root),
        };
        #[cfg(any(not(debug_assertions), target_arch = "wasm32"))]
        let fs = $crate::EmbeddedOnRelease::Embedded {
            dir: $($tts)*,
        };
        fs
    }};
}

struct ModFs {
    driver: Arc<FsDriver>,
    ignore: Arc<globset::GlobSet>,
    name: String,
}

enum FsDriver {
    Embedded { dir: include_dir::Dir<'static> },
    FileSystem { root: PathBuf },
}

impl ModFs {
    async fn list_mods(self: &Arc<Self>) -> Result<Vec<ModFs>, AssetReaderError> {
        match self.driver.deref() {
            FsDriver::Embedded { dir } => {
                let mut mods = Vec::new();
                for dir in dir.dirs() {
                    mods.push(ModFs {
                        name: utf8_file_name(dir.path())?,
                        driver: self.driver.clone(),
                        ignore: self.ignore.clone(),
                    });
                }

                Ok(mods)
            }
            FsDriver::FileSystem { root } => {
                let mut entries = async_fs::read_dir(root).await?;
                let mut mods = Vec::new();
                while let Some(entry) = entries.try_next().await? {
                    if entry.file_type().await?.is_dir() {
                        mods.push(ModFs {
                            name: utf8_file_name(&entry.path())?,
                            driver: self.driver.clone(),
                            ignore: self.ignore.clone(),
                        });
                    }
                }
                Ok(mods)
            }
        }
    }

    async fn is_directory(&self, path: &Path) -> Result<bool, AssetReaderError> {
        self.check_ignore(path)?;
        match self.driver.deref() {
            FsDriver::Embedded { dir } => Ok(dir.get_dir(path).is_some()),
            FsDriver::FileSystem { root } => {
                let file_path = normalize_with_parent(root, path)?;
                Ok(async_fs::metadata(file_path).await?.is_dir())
            }
        }
    }

    fn check_ignore(&self, path: &Path) -> Result<(), AssetReaderError> {
        if self.ignore.is_match(path) {
            Err(AssetReaderError::NotFound(path.to_owned()))
        } else {
            Ok(())
        }
    }
}

impl Display for ModFs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self.driver.deref() {
            FsDriver::Embedded { .. } => {
                write!(f, "ModFs(<embedded>)")
            }
            FsDriver::FileSystem { root } => {
                write!(f, "ModFs(path: {})", root.display())
            }
        }
    }
}
fn get_meta_path(path: &Path) -> PathBuf {
    let mut meta_path = path.to_path_buf();
    let mut extension = path.extension().unwrap_or_default().to_os_string();
    if !extension.is_empty() {
        extension.push(".");
    }
    extension.push("meta");
    meta_path.set_extension(extension);
    meta_path
}

fn normalize_with_parent(root_path: &Path, child: &Path) -> Result<PathBuf, io::Error> {
    let mut lexical = PathBuf::new();
    for component in child.components() {
        match component {
            Component::RootDir | Component::Prefix(_) => {
                return Err(io::Error::other("Absolute paths are not allowed"));
            }
            Component::ParentDir => {
                if lexical.as_os_str().is_empty() {
                    return Err(io::Error::other("Path cannot go above root"));
                }
                lexical.pop();
            }
            Component::Normal(path) => lexical.push(path),
            Component::CurDir => {}
        }
    }
    Ok(root_path.join(lexical))
}
