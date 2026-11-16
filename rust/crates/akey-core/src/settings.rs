//! 전역 설정. 화면 취향만 저장한다(값과 허용은 금고에 있다).
//! 위치: macOS `~/Library/Application Support/akey/settings.json`, Windows `%APPDATA%\akey\settings.json`

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

pub const UNLOCK_CHOICES: [u32; 4] = [5, 15, 30, 60];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// None = 시스템 언어를 따름
    pub language: Option<Lang>,
    /// 인증 한 번으로 보기/복사를 다시 묻지 않는 시간(분)
    pub unlock_minutes: u32,
    /// 복사할 때 클립보드 기록 앱이 저장하지 않도록 표시
    pub conceal_clipboard: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { language: None, unlock_minutes: 30, conceal_clipboard: true }
    }
}

impl Settings {
    pub fn dir() -> PathBuf {
        // 테스트용 위치 바꾸기는 개발(debug) 빌드에서만 (배포 빌드는 환경변수로 바꿀 수 없음)
        #[cfg(debug_assertions)]
        if let Ok(dir) = std::env::var("AKEY_CONFIG_DIR") {
            return PathBuf::from(dir);
        }
        dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("akey")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("settings.json")
    }

    pub fn exists() -> bool {
        Self::path().exists()
    }

    /// 파일이 없거나 깨져 있으면 기본값
    pub fn load() -> Settings {
        let mut s: Settings = std::fs::read(Self::path())
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_default();
        if !UNLOCK_CHOICES.contains(&s.unlock_minutes) {
            s.unlock_minutes = 30;
        }
        s
    }

    pub fn save(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(Self::dir())?;
        let tmp = Self::dir().join("settings.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).expect("settings serialize"))?;
        std::fs::rename(tmp, Self::path())
    }

    pub fn effective_language(&self) -> Lang {
        self.language.unwrap_or_else(Lang::system_default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let s: Settings = serde_json::from_str(r#"{"language":"en"}"#).unwrap();
        assert_eq!(s.language, Some(Lang::En));
        assert_eq!(s.unlock_minutes, 30);
        assert!(s.conceal_clipboard);
    }
}
