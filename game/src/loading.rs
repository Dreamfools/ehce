use crate::mods::{
    HotReloading, ModData, ModHotReloadMessage, ModLoadErrorMessage, ModLoadedMessage, ModState,
    WantLoadModMessage,
};
use crate::report_error;
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
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use utils::map::HashSet;
use utils::rootcause_ext::AttachField;

pub mod json5_asset_plugin;

pub fn load_last_mod(mut evt: MessageWriter<WantLoadModMessage>) {
    let schema = schemars::schema_for!(ModModel);
    let json_str = serde_json::to_string_pretty(&schema).expect("Schema is serializable");
    let mods_path = bevy::asset::io::file::FileAssetReader::get_base_path().join("mods");
    fs_err::create_dir_all(&mods_path).unwrap();
    fs_err::write(mods_path.join("$schema.json"), json_str).unwrap();
    evt.write(WantLoadModMessage("mod".to_string()));
}

#[derive(Debug, Deserialize, Serialize, Asset, Reflect)]
#[serde(transparent)]
pub struct DatabaseAsset(pub ModModel);
#[derive(Debug)]
pub struct ModLoadingPlugin;

impl Plugin for ModLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SimpleStateObjectPlugin::<_, LoadingStateData>::new(ModState::Loading),
            HotReloadEventsPlugin,
        ))
        .add_systems(
            Update,
            (
                loading_initializer,
                loader.run_if(in_state(ModState::Loading)),
            )
                .chain(),
        )
        .add_systems(OnEnter(ModState::Ready), clear_hot_reload_events)
        .add_systems(
            First,
            (
                asset_tracer,
                hot_reload.run_if(in_state(ModState::Ready)),
                hot_reload_events,
            )
                .chain()
                .in_set(HotReloading),
        );
    }
}

#[derive(Debug, Default, Resource)]
struct LoadingStateData {
    name: String,
    folder_handle: Handle<LoadedFolder>,
    not_ready_handles: Option<HashSet<UntypedAssetId>>,
}

// If multiple mod load events are passed in a frame, only the last one is handled
fn loading_initializer(
    mut evt: MessageReader<WantLoadModMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<ModState>>,
) {
    let Some(evt) = evt.read().last() else {
        return;
    };
    let mod_folder = asset_server.load_folder(&evt.0);
    commands.insert_resource(LoadingStateData {
        name: evt.0.clone(),
        folder_handle: mod_folder,
        not_ready_handles: None,
    });
    next_state.set(ModState::Loading)
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
    match asset_server.load_state(&data.folder_handle) {
        LoadState::NotLoaded => {
            error!("Mod folder appears to be missing from asset server");
            state.set(ModState::Pending);
            err_evt.write(ModLoadErrorMessage(
                "Mod folder appears to be missing from asset server".to_string(),
            ));
            return;
        }
        LoadState::Failed(err) => {
            error!("Failed to load mod files: {}", err);
            state.set(ModState::Pending);
            err_evt.write(ModLoadErrorMessage(format!(
                "Failed to load mod files:\n{}",
                err
            )));
            return;
        }
        _ => {}
    }
    let Some(folder) = folder_assets.get(&data.folder_handle) else {
        return;
    };

    let handles = data
        .not_ready_handles
        .get_or_insert_with(|| folder.handles.iter().map(|e| e.id()).collect());

    let mut errors = Vec::new();
    handles.retain(|id| match asset_server.load_state(*id) {
        LoadState::Loaded => false,
        LoadState::Failed(err) => {
            asset_server.get_path(*id);
            errors.push((*id, err));
            true
        }
        _ => true,
    });

    if !errors.is_empty() {
        state.set(ModState::Pending);
        let mut err_str = String::new();
        for (id, err) in errors {
            let path_str = asset_server
                .get_path(id)
                .map(|e| e.path().to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("Unknown path for asset id {}", id));
            err_str.push_str(&format!("Failed to load asset {}: {}\n", path_str, err));
        }
        err_evt.write(ModLoadErrorMessage(err_str));
        return;
    }

    if !handles.is_empty() {
        return;
    }

    let delay = if !*first_load_flag {
        *first_load_flag = true;
        30 // Half-a-second slowdown to let assets update
    } else {
        0
    };
    let wait_until = wait_until.get_or_insert(frame.0 + 1 + delay);

    if frame.0 < *wait_until {
        return;
    }

    // Clear all pending asset events to avoid hot reloading all currently loaded files
    db_asset_events.clear();

    let Some(path) = asset_server.get_path(&data.folder_handle) else {
        error!("Mod folder is missing asset path");
        state.set(ModState::Pending);
        err_evt.write(ModLoadErrorMessage(
            "Mod folder is missing asset path".to_string(),
        ));
        return;
    };

    info!("Mod assets are loaded");
    let mut db_files = Vec::new();
    let mut db_images = Vec::new();
    let asset_type_id = TypeId::of::<DatabaseAsset>();
    let image_type_id = TypeId::of::<Image>();
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

    match construct_mod(
        data.name.clone(),
        path.path().to_path_buf(),
        data.folder_handle.clone(),
        db_files,
        db_images,
    ) {
        Ok(data) => {
            info!("Mod is constructed, sending events");
            state.set(ModState::Pending);
            switch_evt.write(ModLoadedMessage(data));
        }
        Err(err) => {
            let err = err.context("Failed to load a mod");
            let err_str = err.to_string();
            report_error(err);
            state.set(ModState::Pending);
            err_evt.write(ModLoadErrorMessage(err_str));
        }
    }
}

pub fn available_mods<'a>(
    folders: impl IntoIterator<Item = impl AsRef<&'a Path>>,
) -> impl Iterator<Item = String> {
    folders
        .into_iter()
        .filter_map(|e| std::fs::read_dir(e.as_ref()).ok())
        .flat_map(|e| {
            e.filter_map(|e| {
                e.ok().and_then(|e| {
                    e.path()
                        .file_name()
                        .and_then(|e| e.to_str().map(|e| e.to_string()))
                })
            })
        })
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
    _hot_reload_event: MessageWriter<InternalHotReloadMessage>,
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
        if !path.path().starts_with(&loaded_mod.mod_path) {
            continue;
        }
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
                load_mod_evt.write(WantLoadModMessage(loaded_mod.name.clone()));
            }
            *buffer_timer = None;
        }
    }
}

#[derive(Debug)]
struct HotReloadEventsPlugin;

impl Plugin for HotReloadEventsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Messages<InternalHotReloadMessage>>();
        app.init_resource::<Messages<ModHotReloadMessage>>();
    }
}

fn hot_reload_events(
    mut evt: MessageReader<InternalHotReloadMessage>,
    mut untyped_event: MessageWriter<ModHotReloadMessage>,
) {
    if !evt.is_empty() {
        untyped_event.write(ModHotReloadMessage);
        evt.clear()
    }
}

fn clear_hot_reload_events(mut untyped_events: ResMut<Messages<ModHotReloadMessage>>) {
    untyped_events.clear();
}

#[derive(Debug, Message)]
pub enum InternalHotReloadMessage {
    Full,
    Single(RawId),
}

fn construct_mod<'a, 'path>(
    mod_name: String,
    mod_path: PathBuf,
    folder_handle: Handle<LoadedFolder>,
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
        name: mod_name,
        registry,
        mod_path,
        folder_handle,
    })
}
