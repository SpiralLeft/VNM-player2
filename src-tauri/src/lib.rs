mod audio_engine;
mod commands;
mod decoder;
mod player;
mod resampler;
mod symphonia_decoder;

use commands::AppState;
use player::Player;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            player: Mutex::new(Player::new()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_file,
            commands::play,
            commands::pause,
            commands::stop,
            commands::seek,
            commands::set_volume,
            commands::get_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
