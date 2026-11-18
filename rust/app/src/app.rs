//! Tauri 앱 조립: 플러그인, 트레이, 팝오버 창 동작, 설정 창

use akey_core::tr;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_positioner::{Position, WindowExt};

use crate::session::{AppState, EVENT_CHANGED, EVENT_HIDDEN};

pub fn run() {
    let store = crate::platform::open_store().unwrap_or_else(|e| {
        eprintln!("akey: {e}");
        std::process::exit(1)
    });

    tauri::Builder::default()
        // 두 번째 실행은 이미 떠 있는 앱의 팝오버를 연다
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_popover(app)))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(AppState::new(store))
        .invoke_handler(tauri::generate_handler![
            crate::commands::snapshot,
            crate::commands::reveal,
            crate::commands::reveal_all,
            crate::commands::set_group,
            crate::commands::set_group_name,
            crate::commands::copy,
            crate::commands::copy_many,
            crate::commands::lock,
            crate::commands::add,
            crate::commands::save_edit,
            crate::commands::remove,
            crate::commands::set_grant,
            crate::commands::set_all_grant,
            crate::commands::revoke_all_grants,
            crate::commands::get_settings,
            crate::commands::set_language,
            crate::commands::set_unlock_minutes,
            crate::commands::set_conceal_clipboard,
            crate::commands::set_autostart,
            crate::commands::open_settings,
            crate::commands::fit_popover,
            crate::commands::hide_popover,
            crate::commands::debug_log,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            crate::tray::create(app.handle())?;

            // 전체 화면 앱이 있는 화면에서도 뜨게 (macOS).
            // 바깥을 눌러도 숨기지 않는다: 메뉴바 아이콘이나 Esc 로만 닫는다 (검색어 등이 남도록)
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(ptr) = window.ns_window() {
                    unsafe { akey_macos::status::make_popover_panel(ptr) };
                }
            }

            app.state::<AppState>().store.maintain();
            crate::session::spawn_ticker(app.handle().clone());

            // 시작 직후 몇 초 동안은 Tauri 내부가 다른 스레드에서 맡긴 일(화면 갱신 등)도 미뤄질 수 있어
            // 10초 동안 이벤트 루프를 짧게 깨워 준다
            #[cfg(target_os = "macos")]
            std::thread::spawn(|| {
                for _ in 0..40 {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    wake();
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("akey: failed to start")
        .run(|_app, event| {
            // 창을 모두 닫아도 트레이 앱은 계속 실행
            if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}

/// Windows: 작업 표시줄은 보통 아래에 있어 트레이 위로 띄운다.
/// (macOS 는 show_popover_now 에서 메뉴바 아이콘 아래에 직접 놓는다)
#[cfg(not(target_os = "macos"))]
pub fn position_popover(window: &tauri::WebviewWindow) {
    let _ = window.move_window_constrained(Position::TrayCenter);
}

/// `AKEY_DEBUG_LOG=/경로` 를 주면 창 동작을 기록한다 (문제 확인용)
pub fn debug(msg: &str) {
    if let Some(path) = std::env::var_os("AKEY_DEBUG_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            let _ = writeln!(f, "{}.{:03} {msg}", t.as_secs(), t.subsec_millis());
        }
    }
}

/// 메뉴바 아이콘 위치는 메인 스레드에서만 읽을 수 있어, 창 띄우기는 항상 메인 스레드에서 한다
pub fn show_popover(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || show_popover_now(&handle));
    wake();
}

/// 메인 스레드에 맡긴 일이 바로 처리되도록 이벤트 루프를 깨운다.
/// (앱이 막 시작했을 때 등 다른 이벤트가 올 때까지 미뤄지는 경우가 있었다)
pub fn wake() {
    #[cfg(target_os = "macos")]
    akey_macos::status::wake_event_loop();
}

fn show_popover_now(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    debug("show_popover");
    let _ = app.emit(EVENT_CHANGED, ());
    // macOS: 활성화하지 않는 패널을 누른 아이콘 아래에 놓고 앞으로 꺼낸다.
    // 앱을 활성화하면 다른 Space 로 넘어가거나, 전체 화면 앱이 있는 화면에서는 창이 안 보일 수 있다.
    #[cfg(target_os = "macos")]
    if let Ok(ptr) = window.ns_window() {
        unsafe {
            akey_macos::status::configure_popover_window(ptr);
            let placed = akey_macos::status::place_popover_below_status_item(ptr);
            debug(&format!("position {placed:?}"));
            akey_macos::status::show_popover_panel(ptr);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        position_popover(&window);
        let _ = window.show();
        let _ = window.set_focus();
    }
    debug(&format!("shown visible={:?} focused={:?}", window.is_visible(), window.is_focused()));
    #[cfg(target_os = "macos")]
    if std::env::var_os("AKEY_DEBUG_LOG").is_some() {
        if let Ok(ptr) = window.ns_window() {
            debug(&format!("nswindow {}", unsafe { akey_macos::status::window_debug(ptr) }));
        }
    }
}

pub fn hide_popover(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    debug("hide_popover");
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        // 창을 닫으면 보이던 값은 다시 가린다 (잠금 해제 시간은 유지)
        let _ = app.emit(EVENT_HIDDEN, ());
    }
}

pub fn toggle_popover(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    debug(&format!("toggle visible={:?}", window.is_visible()));
    if window.is_visible().unwrap_or(false) {
        hide_popover(app);
    } else {
        show_popover(app);
    }
}

pub fn open_settings(app: &AppHandle) {
    hide_popover(app);
    let title = tr("Akey 설정", "Akey Settings", "Akey 设置");
    let window = match app.get_webview_window("settings") {
        Some(window) => {
            let _ = window.set_title(&title);
            window
        }
        None => {
            let built = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
                .title(title)
                .inner_size(480.0, 560.0)
                .resizable(false)
                .minimizable(false)
                .maximizable(false)
                .visible(false)
                .focused(true)
                .build();
            let Ok(window) = built else { return };
            window
        }
    };
    // 방금 누른 곳(마우스가 있는 모니터)의 가운데에 놓고 보여 준다
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        place_on_current_monitor(&w);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    });
    wake();
}

#[cfg(target_os = "macos")]
fn place_on_current_monitor(window: &tauri::WebviewWindow) {
    if let Ok(ptr) = window.ns_window() {
        unsafe { akey_macos::status::center_on_mouse_screen(ptr) };
    }
}

/// Windows: 마우스가 있는 모니터의 작업 영역 가운데
#[cfg(not(target_os = "macos"))]
fn place_on_current_monitor(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let Ok(cursor) = app.cursor_position() else { return };
    let Ok(Some(monitor)) = app.monitor_from_point(cursor.x, cursor.y) else { return };
    let Ok(size) = window.outer_size() else { return };
    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + (area.size.height as i32 - size.height as i32) / 2;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}
