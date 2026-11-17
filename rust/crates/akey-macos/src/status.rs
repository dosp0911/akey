//! 메뉴바 아이콘을 직접 만든다 (Swift 판 NSStatusItem 과 같은 방식).
//!
//! Tauri 의 tray-icon 은 버튼 위에 투명한 뷰를 얹어 마우스 이벤트를 받는데, macOS 26 에서는
//! 그 뷰로 클릭이 오지 않는다. 그래서 버튼 자체의 target/action 으로 클릭을 받는다.
//! 모든 함수는 메인 스레드에서 불러야 한다.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSEventMask, NSEventModifierFlags, NSEventType, NSImage, NSMenu, NSMenuItem, NSPanel, NSResponder, NSScreen, NSWindow,
    NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSData, NSSize, NSString};

/// 메뉴바 아이콘 위치 (화면 왼쪽 위 기준 좌표, 포인트 단위)
#[derive(Debug, Clone, Copy)]
pub struct ItemRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// 아이콘이 있는 화면의 오른쪽 끝 x (팝오버가 화면 밖으로 나가지 않게)
    pub screen_right: f64,
    pub screen_left: f64,
}

pub enum StatusEvent {
    LeftClick,
    RightClick,
    MenuItem(isize),
}

type Callback = Box<dyn Fn(StatusEvent)>;

thread_local! {
    static ITEM: RefCell<Option<Retained<NSStatusItem>>> = const { RefCell::new(None) };
    static HANDLER: RefCell<Option<Retained<Handler>>> = const { RefCell::new(None) };
    static CALLBACK: RefCell<Option<Callback>> = const { RefCell::new(None) };
}

fn emit(event: StatusEvent) {
    CALLBACK.with(|cb| {
        if let Some(cb) = cb.borrow().as_ref() {
            cb(event);
        }
    });
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AkeyStatusItemHandler"]
    struct Handler;

    impl Handler {
        #[unsafe(method(clicked:))]
        fn clicked(&self, _sender: Option<&AnyObject>) {
            let mtm = self.mtm();
            let app = NSApplication::sharedApplication(mtm);
            let right = app.currentEvent().map(|e| {
                e.r#type() == NSEventType::RightMouseUp
                    || e.r#type() == NSEventType::RightMouseDown
                    || e.modifierFlags().contains(NSEventModifierFlags::Control)
            });
            emit(if right.unwrap_or(false) { StatusEvent::RightClick } else { StatusEvent::LeftClick });
        }

        #[unsafe(method(menuItemClicked:))]
        fn menu_item_clicked(&self, item: &NSMenuItem) {
            emit(StatusEvent::MenuItem(item.tag()));
        }
    }
);

impl Handler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// 메뉴바 아이콘을 만든다.
/// - icon_png: 템플릿 아이콘(검은색 + 투명) PNG
/// - autosave_name: 위치를 기억할 이름 (Swift 판과 같은 "Item-0" 이면 정해 둔 위치를 이어받는다)
pub fn create_status_item(icon_png: &[u8], tooltip: &str, autosave_name: &str, callback: Callback) -> bool {
    let Some(mtm) = MainThreadMarker::new() else { return false };
    CALLBACK.with(|cb| *cb.borrow_mut() = Some(callback));

    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    item.setAutosaveName(Some(&NSString::from_str(autosave_name)));

    let handler = Handler::new(mtm);
    if let Some(button) = item.button(mtm) {
        let data = NSData::with_bytes(icon_png);
        match NSImage::initWithData(NSImage::alloc(), &data) {
            Some(image) => {
                image.setSize(NSSize::new(18.0, 18.0));
                image.setTemplate(true);
                button.setImage(Some(&image));
            }
            // 아이콘을 못 읽으면 글자라도 보이게 한다 (폭이 0 이면 메뉴바에 나타나지 않음)
            None => button.setTitle(&NSString::from_str("Akey")),
        }
        button.setToolTip(Some(&NSString::from_str(tooltip)));
        unsafe {
            button.setTarget(Some(&handler));
            button.setAction(Some(sel!(clicked:)));
        }
        button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
    }

    ITEM.with(|i| *i.borrow_mut() = Some(item));
    HANDLER.with(|h| *h.borrow_mut() = Some(handler));
    true
}

/// 아이콘의 화면 위치 (왼쪽 위 기준). 아이콘이 없거나 아직 배치되지 않았으면 None
pub fn status_item_rect() -> Option<ItemRect> {
    let mtm = MainThreadMarker::new()?;
    ITEM.with(|i| {
        let item = i.borrow();
        let button = item.as_ref()?.button(mtm)?;
        let window = button.window()?;
        let frame = window.frame();
        // Cocoa 좌표(주 화면 왼쪽 아래 원점) → 왼쪽 위 원점
        let screens = NSScreen::screens(mtm);
        let primary_height = screens.firstObject().map(|s| s.frame().size.height).unwrap_or(0.0);
        let screen = window.screen().map(|s| s.frame());
        Some(ItemRect {
            x: frame.origin.x,
            y: primary_height - (frame.origin.y + frame.size.height),
            width: frame.size.width,
            height: frame.size.height,
            screen_left: screen.map(|s| s.origin.x).unwrap_or(0.0),
            screen_right: screen.map(|s| s.origin.x + s.size.width).unwrap_or(f64::MAX),
        })
    })
}

/// 오른쪽 클릭 메뉴를 아이콘 아래에 띄운다. items: (tag, 제목). tag 0 은 구분선.
/// 고르면 콜백에 StatusEvent::MenuItem(tag) 가 온다.
pub fn show_menu(items: &[(isize, String)]) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let menu = NSMenu::new(mtm);
    HANDLER.with(|h| {
        let handler = h.borrow();
        for (tag, title) in items {
            if *tag == 0 {
                menu.addItem(&NSMenuItem::separatorItem(mtm));
                continue;
            }
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(title),
                    Some(sel!(menuItemClicked:)),
                    &NSString::from_str(""),
                )
            };
            item.setTag(*tag);
            if let Some(handler) = handler.as_ref() {
                unsafe { item.setTarget(Some(handler)) };
            }
            menu.addItem(&item);
        }
    });
    ITEM.with(|i| {
        if let Some(item) = i.borrow().as_ref() {
            // 잠시 메뉴를 붙여 버튼을 누른 것처럼 띄운 뒤 떼어야 왼쪽 클릭이 다시 팝오버를 연다
            #[allow(deprecated)]
            item.setMenu(Some(&menu));
            if let Some(button) = item.button(mtm) {
                unsafe { button.performClick(None) };
            }
            #[allow(deprecated)]
            item.setMenu(None);
        }
    });
}

define_class!(
    /// 팝오버용 패널. 패널이어야 앱을 활성화하지 않고도 전체 화면 앱 위 등 현재 Space 에 뜬다
    /// (Swift 판 NSPopover 와 같은 성격). 글자 입력을 받을 수 있게 키 창이 될 수 있다.
    #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AkeyPopoverPanel"]
    struct PopoverPanel;

    impl PopoverPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

/// Tauri 가 만든 팝오버 창을 활성화하지 않는 패널로 바꾼다 (한 번만 부르면 된다).
///
/// # Safety
/// `ns_window` 는 살아 있는 NSWindow 여야 하고 메인 스레드에서 불러야 한다.
pub unsafe fn make_popover_panel(ns_window: *mut std::ffi::c_void) {
    use objc2::runtime::AnyObject;
    use objc2::ClassType;
    use objc2_app_kit::NSWindowStyleMask;
    if ns_window.is_null() {
        return;
    }
    AnyObject::set_class(&*(ns_window as *mut AnyObject), PopoverPanel::class());
    let panel = &*(ns_window as *mut NSPanel);
    panel.setStyleMask(panel.styleMask() | NSWindowStyleMask::NonactivatingPanel);
    panel.setFloatingPanel(true);
    panel.setHidesOnDeactivate(false);
    panel.setWorksWhenModal(true);
    configure_popover_window(ns_window);
}

/// 앱을 활성화하지 않고 패널을 앞으로 꺼내 키 창으로 만든다 (Space 를 옮기지 않음)
///
/// # Safety
/// `ns_window` 는 살아 있는 NSWindow 여야 하고 메인 스레드에서 불러야 한다.
pub unsafe fn show_popover_panel(ns_window: *mut std::ffi::c_void) {
    if ns_window.is_null() {
        return;
    }
    let w = &*(ns_window as *mut objc2_app_kit::NSWindow);
    w.orderFrontRegardless();
    w.makeKeyWindow();
}

/// 팝오버 창을 메뉴바 팝오버처럼 만든다.
/// - 모든 Space 에 나타나고, 전체 화면 앱 위에도 뜬다 (그렇지 않으면 전체 화면 앱이 있는 화면에서
///   메뉴바를 눌러도 창이 다른 Space 에 떠서 보이지 않는다)
/// - 메뉴와 같은 높이(레벨)에 둔다
///
/// # Safety
/// `ns_window` 는 살아 있는 NSWindow 여야 하고 메인 스레드에서 불러야 한다.
pub unsafe fn configure_popover_window(ns_window: *mut std::ffi::c_void) {
    use objc2_app_kit::{NSPopUpMenuWindowLevel, NSWindowCollectionBehavior};
    if ns_window.is_null() {
        return;
    }
    let w = &*(ns_window as *mut objc2_app_kit::NSWindow);
    w.setCollectionBehavior(
        w.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    w.setLevel(NSPopUpMenuWindowLevel);
}

/// 팝오버를 방금 누른 메뉴바 아이콘 바로 아래 가운데에 놓는다 (macOS 좌표 그대로, 변환 없음).
/// 모니터마다 메뉴바가 따로 있어 아이콘 위치는 누른 화면에 따라 달라진다. 그 화면 안으로 좌우를 맞춘다.
/// 돌려주는 값: 놓은 위치 설명 (진단 기록용). 아이콘이 없으면 None.
///
/// # Safety
/// `ns_window` 는 살아 있는 NSWindow 여야 하고 메인 스레드에서 불러야 한다.
pub unsafe fn place_popover_below_status_item(ns_window: *mut std::ffi::c_void) -> Option<String> {
    use objc2_foundation::NSPoint;
    let mtm = MainThreadMarker::new()?;
    if ns_window.is_null() {
        return None;
    }
    let popover = &*(ns_window as *mut objc2_app_kit::NSWindow);
    ITEM.with(|i| {
        let item = i.borrow();
        let button = item.as_ref()?.button(mtm)?;
        let item_window = button.window()?;
        let icon = item_window.frame();
        let area = item_window.screen().map(|s| s.visibleFrame()).unwrap_or(icon);
        let width = popover.frame().size.width;
        let x = (icon.origin.x + icon.size.width / 2.0 - width / 2.0)
            .min(area.origin.x + area.size.width - width - 8.0)
            .max(area.origin.x + 8.0);
        // 아이콘 창의 아래쪽 4pt 밑이 팝오버의 위쪽
        let top = icon.origin.y - 4.0;
        popover.setFrameTopLeftPoint(NSPoint::new(x, top));
        Some(format!("icon=({},{}) top_left=({x},{top}) screen=({},{} {}x{})",
            icon.origin.x, icon.origin.y, area.origin.x, area.origin.y, area.size.width, area.size.height))
    })
}

/// 팝오버 내용 높이를 바꾼다. 위쪽 왼쪽 모서리는 그대로 둔다 (아래로만 늘거나 줄어든다).
/// Tauri 의 set_size 는 나중에 따로 적용되고 아래쪽 왼쪽을 기준으로 바뀌어서,
/// 그 뒤에 위치를 다시 맞추는 사이 창이 엉뚱한 곳(화면 구석)으로 튈 수 있었다. 한 번에 직접 바꾼다.
///
/// # Safety
/// `ns_window` 는 살아 있는 NSWindow 여야 하고 메인 스레드에서 불러야 한다.
pub unsafe fn resize_popover_keep_top(ns_window: *mut std::ffi::c_void, width: f64, height: f64) {
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    if ns_window.is_null() {
        return;
    }
    let w = &*(ns_window as *mut objc2_app_kit::NSWindow);
    let old = w.frame();
    let content = w.contentRectForFrameRect(old);
    let frame_for = w.frameRectForContentRect(NSRect::new(content.origin, NSSize::new(width, height)));
    let top = old.origin.y + old.size.height;
    let new = NSRect::new(NSPoint::new(old.origin.x, top - frame_for.size.height), frame_for.size);
    if (new.size.height - old.size.height).abs() > 0.5 || (new.size.width - old.size.width).abs() > 0.5 {
        w.setFrame_display(new, true);
    }
}

/// 문제 확인용: NSWindow 상태를 글로 돌려준다
///
/// # Safety
/// `ns_window` 는 살아 있는 NSWindow 여야 하고 메인 스레드에서 불러야 한다.
pub unsafe fn window_debug(ns_window: *mut std::ffi::c_void) -> String {
    if ns_window.is_null() {
        return "null".into();
    }
    let w = &*(ns_window as *mut objc2_app_kit::NSWindow);
    let f = w.frame();
    let screens = MainThreadMarker::new()
        .map(|m| {
            NSScreen::screens(m)
                .iter()
                .map(|s| {
                    let r = s.frame();
                    format!("[{},{} {}x{} scale={}]", r.origin.x, r.origin.y, r.size.width, r.size.height, s.backingScaleFactor())
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let on = w
        .screen()
        .map(|s| {
            let r = s.frame();
            format!("[{},{} {}x{}]", r.origin.x, r.origin.y, r.size.width, r.size.height)
        })
        .unwrap_or_else(|| "none".into());
    format!(
        "screens={screens} window_screen={on} behavior={:?} isVisible={} onActiveSpace={} occlusion={:?} alpha={} level={} frame=({},{},{},{}) app_active={} app_hidden={}",
        w.collectionBehavior(),
        w.isVisible(),
        w.isOnActiveSpace(),
        w.occlusionState(),
        w.alphaValue(),
        w.level(),
        f.origin.x,
        f.origin.y,
        f.size.width,
        f.size.height,
        MainThreadMarker::new().map(|m| NSApplication::sharedApplication(m).isActive()).unwrap_or(false),
        MainThreadMarker::new().map(|m| NSApplication::sharedApplication(m).isHidden()).unwrap_or(false),
    )
}

/// 메인 스레드 이벤트 루프를 깨운다 (어느 스레드에서 불러도 된다).
/// 앱에 빈 이벤트(ApplicationDefined) 하나를 넣어, 다른 스레드가 맡긴 일을 바로 처리하게 한다.
pub fn wake_event_loop() {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_foundation::NSPoint;
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        if app.is_null() {
            return;
        }
        let event: *mut AnyObject = msg_send![
            class!(NSEvent),
            otherEventWithType: 15usize, // NSEventTypeApplicationDefined
            location: NSPoint::new(0.0, 0.0),
            modifierFlags: 0usize,
            timestamp: 0.0f64,
            windowNumber: 0isize,
            context: std::ptr::null_mut::<AnyObject>(),
            subtype: 0i16,
            data1: 0isize,
            data2: 0isize
        ];
        if event.is_null() {
            return;
        }
        let _: () = msg_send![app, postEvent: event, atStart: false];
    }
}

/// 표준 "정보" 창
pub fn show_about() {
    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        app.orderFrontStandardAboutPanel(None);
    }
}

#[allow(dead_code)]
fn _assert_any_thread() {
    // AllocAnyThread 는 NSData 등 생성에 필요한 트레이트 (사용 확인용)
    let _ = NSData::alloc();
}
