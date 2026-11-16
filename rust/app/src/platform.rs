//! 플랫폼별 저장소·인증 전 준비·클립보드

use std::sync::Arc;

use akey_core::{SecretStore, Settings};

pub fn init() {
    #[cfg(target_os = "macos")]
    {
        akey_macos::init_system_language();
        akey_macos::import_legacy_settings();
    }
    #[cfg(windows)]
    akey_windows::init_system_language();
    akey_core::i18n::set_lang(Settings::load().effective_language());
}

pub fn open_store() -> akey_core::Result<Arc<dyn SecretStore>> {
    #[cfg(target_os = "macos")]
    return Ok(Arc::new(akey_macos::KeychainStore::new()));
    #[cfg(windows)]
    return Ok(Arc::new(akey_windows::FileStore::open()?));
    #[cfg(not(any(target_os = "macos", windows)))]
    return Err(akey_core::Error::Backend("unsupported platform".into()));
}

/// 인증 창(Touch ID 등)이 앞에 뜨도록 앱을 활성화한다. 메인 스레드에서 불러야 한다.
pub fn activate_for_auth() {
    #[cfg(target_os = "macos")]
    akey_macos::activate_app();
}

/// 복사한 값은 클립보드에 계속 남는다. conceal 이면 클립보드 기록 앱이 저장하지 않도록 표시한다.
pub fn copy_to_clipboard(value: &str, conceal: bool) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let set = clipboard.set();
    #[cfg(target_os = "macos")]
    let set = {
        use arboard::SetExtApple;
        if conceal { set.exclude_from_history() } else { set }
    };
    #[cfg(windows)]
    let set = {
        use arboard::SetExtWindows;
        if conceal { set.exclude_from_history().exclude_from_cloud().exclude_from_monitoring() } else { set }
    };
    #[cfg(not(any(target_os = "macos", windows)))]
    let _ = conceal;
    set.text(value.to_string()).map_err(|e| e.to_string())
}
