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
    /// 사용자가 마우스로 정한 팝오버 크기. None = 내용에 맞춤
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popover_size: Option<PopoverSize>,
}

/// 팝오버 크기 (논리 픽셀)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PopoverSize {
    pub width: f64,
    pub height: f64,
}

impl PopoverSize {
    pub const MIN_WIDTH: f64 = 470.0;
    pub const MAX_WIDTH: f64 = 1400.0;
    pub const MIN_HEIGHT: f64 = 220.0;
    pub const MAX_HEIGHT: f64 = 1600.0;

    /// 범위 밖이나 숫자가 아닌 값은 범위 안으로 맞춘다
    pub fn clamped(width: f64, height: f64) -> PopoverSize {
        let fix = |v: f64, lo: f64, hi: f64| if v.is_finite() { v.round().clamp(lo, hi) } else { lo };
        PopoverSize {
            width: fix(width, Self::MIN_WIDTH, Self::MAX_WIDTH),
            height: fix(height, Self::MIN_HEIGHT, Self::MAX_HEIGHT),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings { language: None, unlock_minutes: 30, conceal_clipboard: true, popover_size: None }
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
        s.popover_size = s.popover_size.map(|p| PopoverSize::clamped(p.width, p.height));
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

    #[test]
    fn popover_size_round_trip_and_clamp() {
        // 저장하지 않았으면 파일에 쓰지 않고, 읽으면 None
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(!json.contains("popover_size"));
        let s: Settings = serde_json::from_str(r#"{"popover_size":{"width":700,"height":500}}"#).unwrap();
        assert_eq!(s.popover_size, Some(PopoverSize { width: 700.0, height: 500.0 }));
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.popover_size, s.popover_size);

        assert_eq!(PopoverSize::clamped(10.0, 99999.0), PopoverSize { width: 470.0, height: 1600.0 });
        assert_eq!(PopoverSize::clamped(f64::NAN, 300.4), PopoverSize { width: 470.0, height: 300.0 });
    }
}
