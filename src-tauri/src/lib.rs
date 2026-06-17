// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            window.set_decorations(false)?;
            window.set_skip_taskbar(true)?;
            window.set_ignore_cursor_events(true)?;

            window.set_resizable(false)?;
            window.show()?;

            // 构建托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示/隐藏", true, None::<&str>)?;
            let music_off =
                CheckMenuItem::with_id(app, "music-off", "🔇 关闭音乐", true, false, None::<&str>)?;
            let music1 =
                CheckMenuItem::with_id(app, "music-1", "🎵 音乐一", true, true, None::<&str>)?;
            let music2 =
                CheckMenuItem::with_id(app, "music-2", "🎵 音乐二", true, false, None::<&str>)?;
            let music_submenu =
                Submenu::with_items(app, "背景音乐", true, &[&music_off, &music1, &music2])?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &music_submenu, &quit_item])?;

            // 创建系统托盘图标
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event({
                    let music_off = music_off.clone();
                    let music1 = music1.clone();
                    let music2 = music2.clone();
                    move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                                let _ = app.emit("snow-stop", ());
                                let _ = app.emit("music-stop", ());
                            } else {
                                let _ = window.show();
                                let _ = window.set_ignore_cursor_events(true);
                                let _ = app.emit("snow-restart", ());
                                let _ = app.emit("music-play", ());
                            }
                        }
                    }
                    "music-off" => {
                        let _ = music_off.set_checked(true);
                        let _ = music1.set_checked(false);
                        let _ = music2.set_checked(false);
                        let _ = app.emit("music-stop", ());
                    }
                    "music-1" => {
                        let _ = music_off.set_checked(false);
                        let _ = music1.set_checked(true);
                        let _ = music2.set_checked(false);
                        let _ = app.emit("music-select", "music-1");
                    }
                    "music-2" => {
                        let _ = music_off.set_checked(false);
                        let _ = music1.set_checked(false);
                        let _ = music2.set_checked(true);
                        let _ = app.emit("music-select", "music-2");
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }})
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                                let _ = app.emit("snow-stop", ());
                                let _ = app.emit("music-stop", ());
                            } else {
                                let _ = window.show();
                                let _ = window.set_ignore_cursor_events(true);
                                let _ = app.emit("snow-restart", ());
                                let _ = app.emit("music-play", ());
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
