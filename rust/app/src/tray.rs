//! 트레이(메뉴바) 아이콘. 왼쪽 클릭 = 팝오버, 오른쪽 클릭 = 정보 / 설정… / 종료
//!
//! - macOS: NSStatusItem 을 직접 만든다 (akey_macos::status). Tauri tray-icon 은 macOS 26 에서
//!   클릭을 받지 못한다.
//! - Windows: Tauri 트레이

use akey_core::tr;
use tauri::AppHandle;

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use akey_macos::status::{self, StatusEvent};

    const MENU_ABOUT: isize = 1;
    const MENU_SETTINGS: isize = 2;
    const MENU_QUIT: isize = 3;

    pub fn create(app: &AppHandle) -> tauri::Result<()> {
        let handle = app.clone();
        let created = status::create_status_item(
            include_bytes!("../icons/tray-template.png"),
            "Akey",
            // Swift 판과 같은 이름: 이미 정해 둔 메뉴바 위치를 이어받는다
            "Item-0",
            Box::new(move |event| match event {
                StatusEvent::LeftClick => {
                    crate::app::debug("status left click");
                    crate::app::toggle_popover(&handle);
                }
                StatusEvent::RightClick => {
                    crate::app::debug("status right click");
                    crate::app::hide_popover(&handle);
                    status::show_menu(&[
                        (MENU_ABOUT, tr("Akey 정보", "About Akey")),
                        (MENU_SETTINGS, tr("설정…", "Settings…")),
                        (0, String::new()),
                        (MENU_QUIT, tr("종료", "Quit")),
                    ]);
                }
                StatusEvent::MenuItem(MENU_ABOUT) => status::show_about(),
                StatusEvent::MenuItem(MENU_SETTINGS) => crate::app::open_settings(&handle),
                StatusEvent::MenuItem(MENU_QUIT) => handle.exit(0),
                StatusEvent::MenuItem(_) => {}
            }),
        );
        crate::app::debug(&format!("status item created={created}"));
        Ok(())
    }

    /// 메뉴는 오른쪽 클릭 때마다 현재 언어로 새로 만든다
    pub fn refresh_menu(_app: &AppHandle) {}
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;
    use tauri::image::Image;
    use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::Wry;

    const TRAY_ID: &str = "akey-tray";

    fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
        let about = PredefinedMenuItem::about(
            app,
            Some(&tr("Akey 정보", "About Akey")),
            Some(AboutMetadata {
                name: Some("Akey".into()),
                version: Some(app.package_info().version.to_string()),
                comments: Some(tr(
                    "에이전트용 로컬 시크릿 저장소.\n값은 이 컴퓨터에만 저장됩니다.",
                    "A local secret store for AI agents.\nValues are kept only on this computer.",
                )),
                ..Default::default()
            }),
        )?;
        let settings = MenuItem::with_id(app, "settings", tr("설정…", "Settings…"), true, None::<&str>)?;
        let separator = PredefinedMenuItem::separator(app)?;
        let quit = MenuItem::with_id(app, "quit", tr("종료", "Quit"), true, None::<&str>)?;
        Menu::with_items(app, &[&about, &settings, &separator, &quit])
    }

    pub fn create(app: &AppHandle) -> tauri::Result<()> {
        let menu = build_menu(app)?;
        TrayIconBuilder::with_id(TRAY_ID)
            .icon(Image::from_bytes(include_bytes!("../icons/32x32.png"))?)
            .tooltip("Akey")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "settings" => crate::app::open_settings(app),
                "quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                    crate::app::toggle_popover(tray.app_handle());
                }
            })
            .build(app)?;
        Ok(())
    }

    /// 언어를 바꾸면 메뉴 문구도 바꾼다
    pub fn refresh_menu(app: &AppHandle) {
        if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

pub use imp::{create, refresh_menu};
