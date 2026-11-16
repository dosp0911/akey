//! akey Windows 저장소: `%LOCALAPPDATA%\akey\vault.json` + DPAPI + Windows Hello.
//! 금고 로직은 [`engine`] (운영체제와 무관, macOS 에서도 테스트), Windows 호출은 `win` 에 있다.

pub mod engine;

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::{ignore_ctrl_c, init_system_language};

#[cfg(windows)]
pub type FileStore = engine::Engine<win::WinPlatform>;

#[cfg(windows)]
impl engine::Engine<win::WinPlatform> {
    /// 개발(debug) 빌드에서만 `AKEY_VAULT_PATH` 로 다른 파일을 쓰게 할 수 있다 (테스트용)
    pub fn open() -> akey_core::Result<Self> {
        #[cfg(debug_assertions)]
        let override_path = std::env::var_os("AKEY_VAULT_PATH");
        #[cfg(not(debug_assertions))]
        let override_path: Option<std::ffi::OsString> = None;
        let path = match override_path {
            Some(p) => std::path::PathBuf::from(p),
            None => dirs::data_local_dir()
                .ok_or_else(|| akey_core::Error::Backend("LOCALAPPDATA not found".into()))?
                .join("akey")
                .join("vault.json"),
        };
        Ok(engine::Engine::new(win::WinPlatform, path))
    }
}
