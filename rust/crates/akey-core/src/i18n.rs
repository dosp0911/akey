//! 화면·CLI 출력 언어. 문구를 쓰는 자리에 세 언어(한국어, 영어, 중국어 간체)를 함께 둔다: `tr(ko, en, zh)`.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Ko,
    En,
    /// 중국어 간체
    Zh,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        match code {
            "ko" => Some(Lang::Ko),
            "en" => Some(Lang::En),
            "zh" => Some(Lang::Zh),
            _ => None,
        }
    }

    /// 시스템 언어가 한국어면 한국어, 중국어면 중국어, 아니면 영어
    pub fn system_default() -> Lang {
        let candidates = [
            std::env::var("AKEY_SYSTEM_LANG").ok(),
            system_language(),
            std::env::var("LANG").ok(),
        ];
        for c in candidates.into_iter().flatten() {
            if !c.is_empty() {
                let c = c.to_lowercase();
                return if c.starts_with("ko") {
                    Lang::Ko
                } else if c.starts_with("zh") {
                    Lang::Zh
                } else {
                    Lang::En
                };
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

// 0 = 아직 정하지 않음, 1 = ko, 2 = en, 3 = zh
static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    let n = match lang {
        Lang::Ko => 1,
        Lang::En => 2,
        Lang::Zh => 3,
    };
    CURRENT.store(n, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Ko,
        2 => Lang::En,
        3 => Lang::Zh,
        _ => {
            let l = crate::settings::Settings::load().effective_language();
            set_lang(l);
            l
        }
    }
}

/// 현재 언어에 맞는 문구를 고른다.
pub fn tr(ko: &str, en: &str, zh: &str) -> String {
    match lang() {
        Lang::Ko => ko.to_string(),
        Lang::En => en.to_string(),
        Lang::Zh => zh.to_string(),
    }
}
