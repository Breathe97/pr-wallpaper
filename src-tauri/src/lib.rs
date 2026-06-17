// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
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
            // 确保无边框
            window.set_decorations(false)?;
            // 隐藏任务栏图标
            window.set_skip_taskbar(true)?;
            // 启用鼠标事件穿透
            window.set_ignore_cursor_events(true)?;
            // 铺满整个显示器
            if let Some(monitor) = window.current_monitor()? {
                let size = *monitor.size();
                let pos = *monitor.position();
                window.set_position(pos)?;
                window.set_size(tauri::Size::Physical(size))?;
            }
            // 移除 Windows 11 圆角
            #[cfg(target_os = "windows")]
            {
                let hwnd = window.hwnd()?;
                let preference: u32 = 1u32;
                unsafe {
                    windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
                        hwnd.0,
                        33u32,
                        &preference as *const _ as *const std::ffi::c_void,
                        std::mem::size_of::<u32>() as u32,
                    );
                }
            }
            #[cfg(target_os = "windows")]
            window.set_shadow(false)?;
            // 把窗口放到最底层（Wallpaper 层级）
            #[cfg(target_os = "windows")]
            {
                let hwnd = window.hwnd()?;
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                        hwnd.0,
                        windows_sys::Win32::UI::WindowsAndMessaging::HWND_BOTTOM,
                        0,
                        0,
                        0,
                        0,
                        0x0001 | 0x0002,
                    );
                }
            }
            window.set_resizable(false)?;

            // 禁止窗口获得焦点，彻底去掉焦点指示器
            #[cfg(target_os = "windows")]
            {
                let hwnd = window.hwnd()?;
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::*;
                    let ex = GetWindowLongW(hwnd.0, GWL_EXSTYLE);
                    SetWindowLongW(hwnd.0, GWL_EXSTYLE, ex | 0x08000000); // WS_EX_NOACTIVATE
                }
            }

            // 所有设置完成后，显示窗口
            window.show()?;

            // 构建托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示/隐藏", true, None::<&str>)?;
            let music_item =
                CheckMenuItem::with_id(app, "music", "背景音乐", true, true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &music_item, &quit_item])?;

            // 创建系统托盘图标
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
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
                    "music" => {
                        let _ = app.emit("music-toggle", ());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
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
