mod audio_engine;
mod commands;
mod decoder;
mod nwa_decoder;
mod player;
mod playlist;
mod resampler;
mod symphonia_decoder;

use commands::AppState;
use player::Player;
use playlist::Playlist;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            player: Mutex::new(Player::new()),
            playlist: Mutex::new(Playlist::new()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_file,
            commands::play,
            commands::pause,
            commands::stop,
            commands::seek,
            commands::set_volume,
            commands::get_state,
            commands::add_to_playlist,
            commands::remove_from_playlist,
            commands::play_from_playlist,
            commands::next_track,
            commands::previous_track,
            commands::get_playlist,
            commands::set_loop_mode,
            commands::scan_folder,
            commands::clear_playlist,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
