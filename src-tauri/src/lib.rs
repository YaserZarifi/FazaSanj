mod commands;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![commands::drives::list_drives])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fazasanj failed to start: {e}");
            std::process::exit(1);
        });
}
