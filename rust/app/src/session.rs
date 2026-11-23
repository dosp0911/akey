//! 앱 상태: 저장소, 잠금 해제 세션, 설정.
//!
//! 잠금 해제 세션 = 인증 한 번으로 정한 시간(5~60분) 동안 보기·복사를 다시 묻지 않음.
//! 삭제·수정(이름·값)·덮어쓰기·허용 변경은 세션과 상관없이 매번 인증한다 (Swift 판과 같음).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use akey_core::{SecretStore, Settings};
use tauri::{AppHandle, Emitter, Manager};

pub const EVENT_CHANGED: &str = "akey://changed";
pub const EVENT_HIDDEN: &str = "akey://hidden";

pub struct AppState {
    pub store: Arc<dyn SecretStore>,
    inner: Mutex<Inner>,
}

struct Inner {
    unlocked_until: Option<Instant>,
    settings: Settings,
}

impl AppState {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        AppState {
            store,
            inner: Mutex::new(Inner {
                unlocked_until: None,
                settings: Settings::load(),
            }),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn settings(&self) -> Settings {
        self.inner().settings.clone()
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) -> std::io::Result<()> {
        let mut inner = self.inner();
        f(&mut inner.settings);
        inner.settings.save()
    }

    /// 끄는 동안(드래그 중)은 메모리에만 두고, 놓을 때(persist) 파일에 쓴다
    pub fn set_popover_size(&self, size: Option<akey_core::PopoverSize>, persist: bool) -> std::io::Result<()> {
        let mut inner = self.inner();
        inner.settings.popover_size = size;
        if persist {
            inner.settings.save()?;
        }
        Ok(())
    }

    pub fn is_unlocked(&self) -> bool {
        matches!(self.inner().unlocked_until, Some(t) if t > Instant::now())
    }

    pub fn minutes_left(&self) -> Option<u64> {
        let until = self.inner().unlocked_until?;
        let left = until.checked_duration_since(Instant::now())?;
        Some(left.as_secs().div_ceil(60))
    }

    /// 세션을 끝내고 저장소가 기억한 키도 지운다 (Windows Hello 키)
    pub fn lock(&self) {
        self.inner().unlocked_until = None;
        self.store.lock();
    }

    /// 인증 창을 띄운다. 앱을 앞으로 꺼낸 뒤(Touch ID 가 뜨게) 다른 스레드에서 기다린다.
    pub async fn authenticate(&self, app: &AppHandle, reason: String) -> bool {
        // 인증 창이 닫히면 macOS 가 이전에 쓰던 앱으로 포커스를 돌려, akey 창(특히 설정 창)이
        // 뒤로 사라진 것처럼 보였다. 앞에 있던 akey 창을 기억했다가 인증 뒤 다시 앞으로 꺼낸다.
        let front = ["settings", "main"].into_iter().find(|label| {
            app.get_webview_window(label)
                .map(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false))
                .unwrap_or(false)
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = app.run_on_main_thread(move || {
            crate::platform::activate_for_auth();
            let _ = tx.send(());
        });
        crate::app::wake();
        let _ = tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(2))).await;

        let store = self.store.clone();
        let ok = tauri::async_runtime::spawn_blocking(move || store.authenticate(&reason)).await.unwrap_or(false);

        match front {
            Some("settings") => {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = app.run_on_main_thread(crate::platform::activate_for_auth);
                    let _ = window.show();
                    let _ = window.set_focus();
                    crate::app::wake();
                }
            }
            _ => {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.set_focus();
                        crate::app::wake();
                    }
                }
            }
        }
        ok
    }

    /// 세션이 살아 있으면 바로 통과, 아니면 인증하고 설정한 시간 동안 잠금 해제
    pub async fn unlock(&self, app: &AppHandle, reason: String) -> bool {
        if self.is_unlocked() {
            return true;
        }
        if !self.authenticate(app, reason).await {
            return false;
        }
        let minutes = self.inner().settings.unlock_minutes as u64;
        self.inner().unlocked_until = Some(Instant::now() + Duration::from_secs(minutes * 60));
        let _ = app.emit(EVENT_CHANGED, ());
        true
    }

    /// 매번 묻는 작업용 인증. 세션이 없으면 작업 뒤 저장소 키를 다시 잠근다 (`relock` 을 불러야 함)
    pub async fn confirm(&self, app: &AppHandle, reason: String) -> bool {
        self.authenticate(app, reason).await
    }

    pub fn relock_if_no_session(&self) {
        if !self.is_unlocked() {
            self.store.lock();
        }
    }
}

/// 세션 만료와 허용 만료를 15초마다 확인한다
pub fn spawn_ticker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut was_unlocked = false;
        let mut ticks: u64 = 0;
        loop {
            tokio_sleep(Duration::from_secs(15)).await;
            ticks += 1;
            let state = app.state::<AppState>();
            let unlocked = state.is_unlocked();
            if was_unlocked && !unlocked {
                state.lock();
                let _ = app.emit(EVENT_CHANGED, ());
            }
            was_unlocked = unlocked;
            // 1분마다: 허용 만료 정리(Windows 는 만료된 값을 다시 Hello 로 봉인) + 남은 시간 표시 갱신
            if ticks % 4 == 0 {
                let store = state.store.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || store.maintain()).await;
                let _ = app.emit(EVENT_CHANGED, ());
            }
        }
    });
}

async fn tokio_sleep(d: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}
