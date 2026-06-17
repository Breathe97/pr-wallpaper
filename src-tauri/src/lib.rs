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
            window.set_decorations(false)?;
            window.set_skip_taskbar(true)?;
            window.set_ignore_cursor_events(true)?;

            // 铺满整个显示器
            if let Some(monitor) = window.current_monitor()? {
                let size = *monitor.size();
                let pos = *monitor.position();
                window.set_position(pos)?;
                window.set_size(tauri::Size::Physical(size))?;
            }

            // 移除 Windows 11 圆角 + 阴影
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
            window.set_resizable(false)?;

            // 设置为 WorkerW 的子窗口 → 随桌面显示/隐藏，Win+D 时可见
            #[cfg(target_os = "windows")]
            {
                let hwnd = window.hwnd()?;
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::*;
                    let progman = FindWindowW(
                        [80u16, 114, 111, 103, 109, 97, 110, 0].as_ptr(),
                        std::ptr::null(),
                    );
                    if !progman.is_null() {
                        // 让 Progman 创建桌面 WorkerW
                        SendMessageW(progman, 0x052C, 0, 0);
                        // 找到包含桌面图标的 WorkerW
                        let mut workerw = std::ptr::null_mut();
                        loop {
                            workerw = FindWindowExW(
                                std::ptr::null_mut(),
                                workerw,
                                [87u16, 111, 114, 107, 101, 114, 87, 0].as_ptr(),
                                std::ptr::null(),
                            );
                            if workerw.is_null() {
                                break;
                            }
                            let view = FindWindowExW(
                                workerw,
                                std::ptr::null_mut(),
                                [
                                    83u16, 72, 69, 76, 76, 68, 76, 76, 95, 68, 101, 102, 86, 105,
                                    101, 119, 0,
                                ]
                                .as_ptr(),
                                std::ptr::null(),
                            );
                            if !view.is_null() {
                                break;
                            }
                        }
                        if !workerw.is_null() {
                            // 设置为 WorkerW 的子窗口
                            SetParent(hwnd.0 as *mut std::ffi::c_void, workerw);
                            // 显示窗口
                            ShowWindow(hwnd.0, 5); // SW_SHOW
                        }
                    }
                }
            }

            #[cfg(not(target_os = "windows"))]
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
