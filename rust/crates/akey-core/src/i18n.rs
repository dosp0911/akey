//! 화면·CLI 출력 언어. Swift 판의 `tr(ko, en)` 과 같은 방식으로, 문구를 쓰는 자리에 두 언어를 함께 둔다.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Ko,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        match code {
            "ko" => Some(Lang::Ko),
            "en" => Some(Lang::En),
            _ => None,
        }
    }

    /// 시스템 언어가 한국어면 한국어, 아니면 영어 (Swift 판과 같은 규칙)
    pub fn system_default() -> Lang {
        let candidates = [
            std::env::var("AKEY_SYSTEM_LANG").ok(),
            system_language(),
            std::env::var("LANG").ok(),
        ];
        for c in candidates.into_iter().flatten() {
            if !c.is_empty() {
                return if c.to_lowercase().starts_with("ko") { Lang::Ko } else { Lang::En };
            }
        }
        Lang::En
    }
}

/// 운영체제의 첫 번째 선호 언어. 각 플랫폼 크레이트가 시작할 때 채운다.
static SYSTEM_LANG: std::sync::OnceLock<String> = std::sync::OnceLock::new();

pub fn set_system_language(code: &str) {
    let _ = SYSTEM_LANG.set(code.to_string());
}

fn system_language() -> Option<String> {
    SYSTEM_LANG.get().cloned()
}

// 0 = 아직 정하지 않음, 1 = ko, 2 = en
static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    CURRENT.store(if lang == Lang::Ko { 1 } else { 2 }, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Ko,
        2 => Lang::En,
        _ => {
            let l = crate::settings::Settings::load().effective_language();
            set_lang(l);
            l
        }
    }
}

/// 현재 언어에 맞는 문구를 고른다.
pub fn tr(ko: &str, en: &str) -> String {
    match lang() {
        Lang::Ko => ko.to_string(),
        Lang::En => en.to_string(),
    }
}
