use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::player::{FileMetadata, Player, PlayerState};

pub struct AppState {
    pub player: Mutex<Player>,
}

#[derive(serde::Serialize)]
pub struct PlayerStateResponse {
    pub state: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub current_file: Option<String>,
    pub volume: f64,
}

#[derive(serde::Serialize)]
pub struct FileMetadataResponse {
    pub path: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub device_sample_rate: u32,
}

#[tauri::command]
pub fn open_file(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<FileMetadataResponse, String> {
    let mut player = state.inner().player.lock().unwrap();
    player.set_app_handle(app);
    let meta: FileMetadata = player.open_file(&path)?;
    Ok(FileMetadataResponse {
        path: meta.path,
        duration_ms: meta.duration_ms,
        sample_rate: meta.file_sample_rate,
        channels: meta.file_channels,
        device_sample_rate: meta.device_sample_rate,
    })
}

#[tauri::command]
pub fn play(state: State<'_, AppState>) -> Result<(), String> {
    let mut player = state.inner().player.lock().unwrap();
    player.play()
}

#[tauri::command]
pub fn pause(state: State<'_, AppState>) -> Result<(), String> {
    let mut player = state.inner().player.lock().unwrap();
    player.pause()
}

#[tauri::command]
pub fn stop(state: State<'_, AppState>) -> Result<(), String> {
    let mut player = state.inner().player.lock().unwrap();
    player.stop()
}

#[tauri::command]
pub fn seek(state: State<'_, AppState>, position_ms: u64) -> Result<(), String> {
    let mut player = state.inner().player.lock().unwrap();
    player.seek(position_ms)
}

#[tauri::command]
pub fn set_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> {
    let player = state.inner().player.lock().unwrap();
    player.set_volume(volume);
    Ok(())
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> Result<PlayerStateResponse, String> {
    let player = state.inner().player.lock().unwrap();
    let meta = player.file_meta();
    Ok(PlayerStateResponse {
        state: match player.state() {
            PlayerState::Idle => "idle",
            PlayerState::Loaded => "paused",
            PlayerState::Playing => "playing",
            PlayerState::Paused => "paused",
        }
        .to_string(),
        position_ms: player.position_ms(),
        duration_ms: meta.as_ref().map(|m| m.duration_ms).unwrap_or(0),
        current_file: meta.map(|m| m.path),
        volume: 1.0,
    })
}
