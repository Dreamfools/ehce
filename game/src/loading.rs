use crate::mods::{
    HotReloading, ModData, ModLoadErrorMessage, ModLoadedMessage, ModState, WantLoadModMessage,
};
use crate::state::SimpleStateObjectPlugin;
use bevy::app::{App, First, Plugin, Update};
use bevy::asset::{
    Asset, AssetEvent, AssetServer, Assets, Handle, LoadState, LoadedFolder, UntypedAssetId,
    UntypedHandle,
};
use bevy::diagnostic::FrameCount;
use bevy::image::Image;
use bevy::log::{error, info};
use bevy::prelude::{
    Commands, IntoScheduleConfigs, Local, Message, MessageReader, MessageWriter, Messages,
    NextState, OnEnter, Query, Res, ResMut, Resource, in_state,
};
use bevy::reflect::Reflect;
use bevy::time::{Time, Timer, TimerMode};
use bevy::window::Window;
use model::ModModel;
use registry::path::FieldPath;
use registry::registry::id::RawId;
use registry::registry::reflect_registry::BuildReflectRegistry;
use rootcause::prelude::ResultExt;
use rootcause::report_collection::ReportCollection;
use rootcause::{IntoReport, Report, bail, report};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::env;
use std::ops::DerefMut;
use std::path::{Display, Path, PathBuf};
use std::sync::LazyLock;
use utils::map::{HashMap, HashSet};
use utils::rootcause_ext::AttachField;

pub mod json5_asset_plugin;

pub static MOD_FOLDER: LazyLock<&'static str> = LazyLock::new(|| {
    if let Ok(mods_dir) = env::var("EHCE_MODS_DIR") {
        return Box::leak(mods_dir.into_boxed_str());
    }
    let base_path = bevy::asset::io::file::FileAssetReader::get_base_path();
    let mods_path = base_path.join("mods");
    Box::leak(
        mods_path
            .to_str()
            .unwrap_or_else(|| panic!("Base path is not valid UTF-8"))
            .to_string()
            .into_boxed_str(),
    )
});

pub fn load_last_mod(mut evt: MessageWriter<WantLoadModMessage>) {
    let schema = schemars::schema_for!(ModModel);
    let json_str = serde_json::to_string_pretty(&schema).expect("Schema is serializable");
    let mods_path = PathBuf::from(*MOD_FOLDER);
    fs_err::create_dir_all(&mods_path).unwrap();
    fs_err::write(mods_path.join("$schema.json"), json_str).unwrap();
    evt.write(WantLoadModMessage);
}

#[derive(Debug, Deserialize, Serialize, Asset, Reflect)]
#[serde(transparent)]
pub struct DatabaseAsset(pub ModModel);
#[derive(Debug)]
pub struct ModLoadingPlugin;

impl Plugin for ModLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SimpleStateObjectPlugin::<_, LoadingStateData>::new(
            ModState::Loading,
        ),))
            .add_systems(
                Update,
                (
                    loading_initializer,
                    loader.run_if(in_state(ModState::Loading)),
                )
                    .chain(),
            )
            .add_systems(
                First,
                (asset_tracer, hot_reload.run_if(in_state(ModState::Ready)))
                    .chain()
                    .in_set(HotReloading),
            );
    }
}

#[derive(Debug, Default, Resource)]
struct LoadingStateData {
    folder_handles: Vec<(String, Handle<LoadedFolder>)>,
    not_ready_handles: HashMap<Handle<LoadedFolder>, HashSet<UntypedAssetId>>,
}

// If multiple mod load events are passed in a frame, only the last one is handled
fn loading_initializer(
    mut evt: MessageReader<WantLoadModMessage>,
    mut err_evt: MessageWriter<ModLoadErrorMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<ModState>>,
) {
    let Some(_) = evt.read().last() else {
        return;
    };

    let mods = match available_mods() {
        Ok(mods) => mods,
        Err(err) => {
            err_evt.write(ModLoadErrorMessage(err));
            return;
        }
    };

    commands.insert_resource(LoadingStateData {
        folder_handles: mods
            .into_iter()
            .map(|name| {
                let handle = asset_server.load_folder(&name);
                (name, handle)
            })
            .collect(),
        not_ready_handles: Default::default(),
    });
    next_state.set(ModState::Loading)
}

fn is_folder_loaded(
    asset_server: &AssetServer,
    folder_assets: &Assets<LoadedFolder>,
    mut not_ready_handles: &mut HashMap<Handle<LoadedFolder>, HashSet<UntypedAssetId>>,
    handle: &Handle<LoadedFolder>,
) -> rootcause::Result<bool> {
    match asset_server.load_state(handle) {
        LoadState::NotLoaded => {
            bail!("Mod folder appears to be missing from asset server");
        }
        LoadState::Failed(err) => {
            error!("Failed to load mod files: {}", err);
            return Err(err.into_report().into_dynamic());
        }
        _ => {}
    }

    let Some(folder) = folder_assets.get(handle) else {
        // bail!("Mod folder asset is missing from asset server");
        return Ok(false);
    };

    let handles = not_ready_handles
        .entry(handle.clone())
        .or_insert_with(|| folder.handles.iter().map(|e| e.id()).collect());

    let mut rg = ReportCollection::new();

    handles.retain(|id| match asset_server.load_state(*id) {
        LoadState::Loaded => false,
        LoadState::Failed(err) => {
            let path = asset_server
                .get_path(*id)
                .map(|p| p.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());

            rg.push(
                err.into_report()
                    .context("Failed to load file")
                    .attach(AttachField("Path", path))
                    .into_cloneable(),
            );
            true
        }
        _ => true,
    });

    if !rg.is_empty() {
        return Err(rg
            .context("One or more mod files failed to load")
            .into_dynamic());
    }

    Ok(handles.is_empty())
}

fn loader(
    asset_server: Res<AssetServer>,
    folder_assets: Res<Assets<LoadedFolder>>,
    database_items: Res<Assets<DatabaseAsset>>,
    images: Res<Assets<Image>>,
    mut db_asset_events: ResMut<Messages<AssetEvent<DatabaseAsset>>>,
    mut data: ResMut<LoadingStateData>,
    mut err_evt: MessageWriter<ModLoadErrorMessage>,
    mut switch_evt: MessageWriter<ModLoadedMessage>,
    frame: Res<FrameCount>,
    mut state: ResMut<NextState<ModState>>,
    mut wait_until: Local<Option<u32>>,
    mut first_load_flag: Local<bool>,
) {
    let mut rg = ReportCollection::new();
    let mut all_loaded = true;
    let data = &mut *data;
    for (mod_name, handle) in &data.folder_handles {
        match is_folder_loaded(
            &asset_server,
            &folder_assets,
            &mut data.not_ready_handles,
            &handle,
        ) {
            Ok(ready) => {
                all_loaded &= ready;
            }
            Err(err) => {
                rg.push(
                    err.context("Failed to load mod")
                        .attach(AttachField("Mod Name", mod_name.clone()))
                        .into_cloneable(),
                );
            }
        }
    }

    if !rg.is_empty() {
        state.set(ModState::Pending);
        err_evt.write(ModLoadErrorMessage(
            rg.context("One or more mod folders failed to load").into(),
        ));
        return;
    }

    if !all_loaded {
        return;
    }

    // let delay = if !*first_load_flag {
    //     *first_load_flag = true;
    //     30 // Half-a-second slowdown to let assets update
    // } else {
    //     0
    // };
    // let wait_until = wait_until.get_or_insert(frame.0 + 1 + delay);
    //
    // if frame.0 < *wait_until {
    //     return;
    // }
    //
    // // Clear all pending asset events to avoid hot reloading all currently loaded files
    // db_asset_events.clear();

    info!("Mod assets are loaded");
    let mut db_files = Vec::new();
    let mut db_images = Vec::new();
    let asset_type_id = TypeId::of::<DatabaseAsset>();
    let image_type_id = TypeId::of::<Image>();
    for (mod_name, handles) in &data.folder_handles {
        let Some(folder) = folder_assets.get(handles) else {
            panic!("Mod folder {} disappeared from asset server", mod_name);
        };

        for handle in &folder.handles {
            match handle.type_id() {
                id if id == asset_type_id => {
                    let Some(item) =
                        database_items.get(&handle.clone().typed_debug_checked::<DatabaseAsset>())
                    else {
                        continue;
                    };
                    let Some(path) = asset_path(&asset_server, handle) else {
                        continue;
                    };

                    db_files.push((path, item));
                }
                id if id == image_type_id
                    && images.contains(&handle.clone().typed_debug_checked::<Image>()) =>
                {
                    let Some(path) = asset_path(&asset_server, handle) else {
                        continue;
                    };
                    db_images.push((path, handle.clone().typed_debug_checked::<Image>()));
                }
                _ => {
                    continue;
                }
            }
        }
    }

    match construct_mod(data.folder_handles.clone(), db_files, db_images) {
        Ok(data) => {
            info!("Mod is constructed, sending events");
            state.set(ModState::Pending);
            switch_evt.write(ModLoadedMessage(data));
        }
        Err(err) => {
            let err = err.context("Failed to load a mod");
            state.set(ModState::Pending);
            err_evt.write(ModLoadErrorMessage(err.into()));
        }
    }
}

pub fn available_mods<'a>() -> rootcause::Result<impl IntoIterator<Item = String>> {
    let mut dirs = vec![];
    for entry in fs_err::read_dir(*MOD_FOLDER)? {
        let entry = entry?;
        let meta = entry
            .metadata()
            .context("Failed to read mod folder entry metadata")
            .attach_with(|| AttachField("Path", entry.path().to_string_lossy().to_string()))?;

        if meta.is_dir() {
            dirs.push(
                entry
                    .file_name()
                    .to_str()
                    .ok_or_else(|| report!("Mod folder entry name is not valid UTF-8"))
                    .attach_with(|| {
                        AttachField("Path", entry.path().to_string_lossy().to_string())
                    })?
                    .to_string(),
            );
        }
    }

    Ok(dirs)
}

fn asset_path(asset_server: &AssetServer, handle: &UntypedHandle) -> Option<PathBuf> {
    let Some(path) = asset_server.get_path(handle.id()) else {
        error!(?handle, id=?handle.id(), "Failed to fetch path for a database item");
        return None;
    };

    Some(path.path().to_path_buf())
}

fn asset_tracer(
    mut folder_evt: MessageReader<AssetEvent<LoadedFolder>>,
    mut asset_evt: MessageReader<AssetEvent<DatabaseAsset>>,
    frame: Res<FrameCount>,
) {
    for evt in folder_evt.read() {
        info!(frame = frame.0, ?evt, "Folder event")
    }
    for evt in asset_evt.read() {
        info!(frame = frame.0, ?evt, "Asset event")
    }
}

fn hot_reload(
    mut evt: MessageReader<AssetEvent<DatabaseAsset>>,
    _asset: Res<Assets<DatabaseAsset>>,
    asset_server: Res<AssetServer>,
    loaded_mod: ResMut<ModData>,
    mut load_mod_evt: MessageWriter<WantLoadModMessage>,
    mut buffer_timer: Local<Option<Timer>>,
    time: Res<Time>,
    windows: Query<&Window>,
) {
    enum Action {
        Add,
        Update,
    }
    let mut want_reload = false;
    for evt in evt.read() {
        let (asset_id, _action) = match evt {
            AssetEvent::Added { id } => (id, Action::Add),
            AssetEvent::Modified { id } => (id, Action::Update),
            AssetEvent::Removed { .. }
            | AssetEvent::LoadedWithDependencies { .. }
            | AssetEvent::Unused { .. } => continue,
        };
        let Some(path) = asset_server.get_path(*asset_id) else {
            continue;
        };
        // if !path.path().starts_with(&loaded_mod.mod_path) {
        //     continue;
        // }
        info!("Item reload is detected, queueing the hot reload.");
        want_reload = true;
        // todo: partial hot reloads?
    }

    if want_reload {
        *buffer_timer = Some(Timer::from_seconds(1.0, TimerMode::Once))
    } else if windows.iter().any(|e| e.focused) {
        if let Some(timer) = buffer_timer.deref_mut() {
            timer.tick(time.elapsed());
            if timer.just_finished() {
                info!("Initializing hot reload");
                load_mod_evt.write(WantLoadModMessage);
            }
            *buffer_timer = None;
        }
    }
}

fn construct_mod<'a, 'path>(
    folder_handles: Vec<(String, Handle<LoadedFolder>)>,
    files: impl IntoIterator<Item = (impl AsRef<Path>, &'a DatabaseAsset)>,
    images: impl IntoIterator<Item = (impl AsRef<Path>, Handle<Image>)>,
) -> rootcause::Result<ModData> {
    let mut reg = BuildReflectRegistry::default();
    reg.expect_singletons(ModModel::required_singletons());

    for (path, img) in images {
        let path: &Path = path.as_ref();
        let path_lossy = path.to_string_lossy().into_owned();
        let id = path
            .file_name()
            .ok_or_else(|| rootcause::report!("Image path has no file name"))
            .and_then(|name| {
                name.to_str()
                    .ok_or_else(|| rootcause::report!("Image file name is not valid UTF-8"))
            })
            .attach_with(|| AttachField("Image path", path_lossy.clone()))?;

        registry::registry::consume_entry::<Handle<Image>>(
            &mut reg,
            &FieldPath::new(&path_lossy),
            RawId::new(id),
            img,
        )
        .context("Failed to add image to registry")
        .attach_with(|| AttachField("Image path", path_lossy.clone()))?;
    }

    for (path, asset) in files {
        let path_lossy = path.as_ref().to_string_lossy().into_owned();
        registry::traverse::traverse(&asset.0, &FieldPath::new(&*path_lossy), &mut reg)
            .context("Failed to traverse mod asset")
            .attach_with(|| AttachField("Path", path_lossy))?;
    }

    let registry = reg.build()?;

    Ok(ModData {
        registry,
        folder_handles,
    })
}
