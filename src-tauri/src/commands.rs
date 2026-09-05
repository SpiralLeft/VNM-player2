use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::player::{FileMetadata, Player, PlayerState};
use crate::playlist::{self, LoopMode, Playlist, PlaylistState};

pub struct AppState {
    pub player: Mutex<Player>,
    pub playlist: Mutex<Playlist>,
    pub cover_cache: Mutex<HashMap<String, Option<String>>>,
}

/// Try to extract cover art from an audio file, returning base64 data URL
fn extract_cover(path: &str) -> Option<String> {
    use base64::Engine;
    use std::path::Path;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::{MetadataOptions, StandardVisualKey};

    let p = Path::new(path);
    let file = std::fs::File::open(p).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probe = symphonia::default::get_probe();
    let mut format = probe.probe(&hint, mss, format_opts, metadata_opts).ok()?;

    if let Some(rev) = format.metadata().skip_to_latest() {
        for visual in &rev.media.visuals {
            if visual.usage == Some(StandardVisualKey::FrontCover)
                || visual.usage == Some(StandardVisualKey::Other)
                || visual.usage.is_none()
            {
                let mime = visual.media_type.as_deref().unwrap_or("image/jpeg");
                let b64 = base64::engine::general_purpose::STANDARD.encode(&visual.data);
                return Some(format!("data:{};base64,{}", mime, b64));
            }
        }
    }
    None
}

fn get_cached_cover(state: &AppState, path: &str) -> Option<String> {
    let mut cache = state.cover_cache.lock().unwrap();
    if let Some(cached) = cache.get(path) {
        return cached.clone();
    }
    let cover = extract_cover(path);
    cache.insert(path.to_string(), cover.clone());
    cover
}

// ── Player commands ──────────────────────────────────────────

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
    // 从播放列表获取循环模式
    let loop_mode = {
        let pl = state.inner().playlist.lock().unwrap();
        pl.loop_mode.clone()
    };
    let mut player = state.inner().player.lock().unwrap();
    player.set_app_handle(app);
    player.set_loop_mode(loop_mode);
    let meta: FileMetadata = player.open_file(&path)?;
    {
        let mut pl = state.inner().playlist.lock().unwrap();
        pl.update_duration(&meta.path, meta.duration_ms);
    }
    Ok(FileMetadataResponse {
        path: meta.path.clone(),
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

// ── Playlist commands ────────────────────────────────────────

#[tauri::command]
pub fn add_to_playlist(state: State<'_, AppState>, path: String) -> Result<PlaylistState, String> {
    let mut pl = state.inner().playlist.lock().unwrap();
    pl.add(path, 0);
    Ok(pl.state())
}

#[tauri::command]
pub fn remove_from_playlist(
    state: State<'_, AppState>,
    index: usize,
) -> Result<PlaylistState, String> {
    let mut pl = state.inner().playlist.lock().unwrap();
    pl.remove(index);
    Ok(pl.state())
}

#[tauri::command]
pub fn play_from_playlist(
    app: AppHandle,
    state: State<'_, AppState>,
    index: usize,
) -> Result<FileMetadataResponse, String> {
    let (path, loop_mode) = {
        let mut pl = state.inner().playlist.lock().unwrap();
        pl.set_current(index);
        let entry = pl.current_entry().cloned();
        let lm = pl.loop_mode.clone();
        (entry.map(|e| e.path), lm)
    };

    let path = path.ok_or_else(|| "No entry at that index".to_string())?;

    let mut player = state.inner().player.lock().unwrap();
    player.set_app_handle(app);
    player.set_loop_mode(loop_mode);
    let meta: FileMetadata = player.open_file(&path)?;
    // Update playlist entry duration
    {
        let mut pl = state.inner().playlist.lock().unwrap();
        pl.update_duration(&meta.path, meta.duration_ms);
    }
    Ok(FileMetadataResponse {
        path: meta.path,
        duration_ms: meta.duration_ms,
        sample_rate: meta.file_sample_rate,
        channels: meta.file_channels,
        device_sample_rate: meta.device_sample_rate,
    })
}

#[tauri::command]
pub fn next_track(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<FileMetadataResponse>, String> {
    let (next_path, loop_mode) = {
        let mut pl = state.inner().playlist.lock().unwrap();
        let idx = pl.next();
        let entry = idx.and_then(|i| pl.entries.get(i)).cloned();
        let lm = pl.loop_mode.clone();
        (entry.map(|e| e.path), lm)
    };

    match next_path {
        Some(path) => {
            let mut player = state.inner().player.lock().unwrap();
            player.set_app_handle(app);
            player.set_loop_mode(loop_mode);
            let meta: FileMetadata = player.open_file(&path)?;
            Ok(Some(FileMetadataResponse {
                path: meta.path,
                duration_ms: meta.duration_ms,
                sample_rate: meta.file_sample_rate,
                channels: meta.file_channels,
                device_sample_rate: meta.device_sample_rate,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn previous_track(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<FileMetadataResponse>, String> {
    let (prev_path, loop_mode) = {
        let mut pl = state.inner().playlist.lock().unwrap();
        let idx = pl.previous();
        let entry = idx.and_then(|i| pl.entries.get(i)).cloned();
        let lm = pl.loop_mode.clone();
        (entry.map(|e| e.path), lm)
    };

    match prev_path {
        Some(path) => {
            let mut player = state.inner().player.lock().unwrap();
            player.set_app_handle(app);
            player.set_loop_mode(loop_mode);
            let meta: FileMetadata = player.open_file(&path)?;
            Ok(Some(FileMetadataResponse {
                path: meta.path,
                duration_ms: meta.duration_ms,
                sample_rate: meta.file_sample_rate,
                channels: meta.file_channels,
                device_sample_rate: meta.device_sample_rate,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn get_playlist(state: State<'_, AppState>) -> Result<PlaylistState, String> {
    let pl = state.inner().playlist.lock().unwrap();
    Ok(pl.state())
}

#[tauri::command]
pub fn set_loop_mode(state: State<'_, AppState>, mode: String) -> Result<PlaylistState, String> {
    let lm = match mode.as_str() {
        "none" => LoopMode::None,
        "single" => LoopMode::Single,
        "list" => LoopMode::List,
        _ => return Err(format!("Invalid loop mode: {}", mode)),
    };
    // 同步到 Player（单曲循环由后端解码线程处理）
    {
        let mut player = state.inner().player.lock().unwrap();
        player.set_loop_mode(lm.clone());
    }
    let mut pl = state.inner().playlist.lock().unwrap();
    pl.set_loop_mode(lm);
    Ok(pl.state())
}

#[tauri::command]
pub fn scan_folder(state: State<'_, AppState>, path: String) -> Result<PlaylistState, String> {
    let files = playlist::scan_folder(&path)?;
    let mut pl = state.inner().playlist.lock().unwrap();
    for (fpath, dur) in files {
        pl.add(fpath, dur);
    }
    Ok(pl.state())
}

#[tauri::command]
pub fn clear_playlist(state: State<'_, AppState>) -> Result<PlaylistState, String> {
    let mut pl = state.inner().playlist.lock().unwrap();
    pl.clear();
    Ok(pl.state())
}

#[tauri::command]
pub fn get_cover_art(state: State<'_, AppState>, path: String) -> Result<Option<String>, String> {
    Ok(get_cached_cover(&state.inner(), &path))
}
