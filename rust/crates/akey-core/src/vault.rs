//! 금고 모델. Swift 판 Store.swift 의 JSON 과 같은 모양이다:
//! `{"secrets":{이름:값}, "grants":{이름:만료시각(epoch 초, 0=무기한)}, "notes":{이름:메모}}`
//! 모르는 필드는 그대로 보존해 Swift 판과 섞어 써도 데이터가 사라지지 않게 한다.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// 전체 허용 키. 지금 있는 시크릿과 앞으로 추가될 시크릿 모두에 적용된다.
/// (이름 규칙상 "*" 는 시크릿 이름이 될 수 없어 겹치지 않는다)
pub const ALL_KEY: &str = "*";
pub const MAX_NOTE_LENGTH: usize = 200;
pub const MAX_NAME_LENGTH: usize = 128;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Vault {
    #[serde(default)]
    pub secrets: BTreeMap<String, String>,
    #[serde(default)]
    pub grants: BTreeMap<String, f64>,
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// 이름 = 환경변수명: 영문/_ 로 시작, 영문/숫자/_ 만, 128자 이하
pub fn is_valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    s.len() <= MAX_NAME_LENGTH && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn check_name(s: &str) -> Result<()> {
    if is_valid_name(s) {
        Ok(())
    } else {
        Err(Error::InvalidName(s.to_string()))
    }
}

/// 줄바꿈·탭은 공백으로, 앞뒤 공백 제거, 200자 제한
pub fn clean_note(note: &str) -> String {
    let flat: String = note
        .chars()
        .map(|c| if c == '\r' || c == '\n' || c == '\t' { ' ' } else { c })
        .collect();
    flat.trim().chars().take(MAX_NOTE_LENGTH).collect()
}

impl Vault {
    pub fn from_json(data: &[u8]) -> Result<Vault> {
        serde_json::from_slice(data).map_err(|e| Error::Corrupt(e.to_string()))
    }

    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("vault serialize")
    }

    pub fn names(&self) -> Vec<String> {
        self.secrets.keys().cloned().collect()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.secrets.contains_key(name)
    }

    pub fn get(&self, name: &str) -> Result<String> {
        check_name(name)?;
        self.secrets.get(name).cloned().ok_or_else(|| Error::NotFound(name.to_string()))
    }

    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        check_name(name)?;
        self.secrets.insert(name.to_string(), value.to_string());
        Ok(())
    }

    /// 같은 이름으로 다시 만들었을 때 예전 허용·메모가 살아나지 않도록 함께 지운다.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        check_name(name)?;
        if self.secrets.remove(name).is_none() {
            return Err(Error::NotFound(name.to_string()));
        }
        self.grants.remove(name);
        self.notes.remove(name);
        Ok(())
    }

    /// 한 번에 수정.
    /// - new: 새 이름 (같으면 유지). 값·메모·개별 허용이 따라간다.
    /// - value: None 이면 값 유지
    /// - note: None 이면 메모 유지, 빈 문자열이면 메모 삭제
    pub fn update(&mut self, old: &str, new: &str, value: Option<&str>, note: Option<&str>) -> Result<()> {
        check_name(old)?;
        check_name(new)?;
        let mut current = self.secrets.get(old).cloned().ok_or_else(|| Error::NotFound(old.to_string()))?;
        if let Some(v) = value {
            current = v.to_string();
        }
        if new != old {
            if self.secrets.contains_key(new) {
                return Err(Error::AlreadyExists(new.to_string()));
            }
            self.secrets.remove(old);
            if let Some(n) = self.notes.remove(old) {
                self.notes.insert(new.to_string(), n);
            }
            if let Some(g) = self.grants.remove(old) {
                self.grants.insert(new.to_string(), g);
            }
        }
        self.secrets.insert(new.to_string(), current);
        if let Some(note) = note {
            self.set_note_unchecked(new, Some(note));
        }
        Ok(())
    }

    pub fn rename(&mut self, old: &str, new: &str) -> Result<()> {
        self.update(old, new, None, None)
    }

    pub fn set_note(&mut self, name: &str, note: Option<&str>) -> Result<()> {
        check_name(name)?;
        if !self.secrets.contains_key(name) {
            return Err(Error::NotFound(name.to_string()));
        }
        self.set_note_unchecked(name, note);
        Ok(())
    }

    fn set_note_unchecked(&mut self, name: &str, note: Option<&str>) {
        let cleaned = clean_note(note.unwrap_or(""));
        if cleaned.is_empty() {
            self.notes.remove(name);
        } else {
            self.notes.insert(name.to_string(), cleaned);
        }
    }

    pub fn set_grant(&mut self, name: &str, until: Option<f64>) -> Result<()> {
        check_name(name)?;
        match until {
            Some(u) => {
                self.grants.insert(name.to_string(), u);
            }
            None => {
                self.grants.remove(name);
            }
        }
        Ok(())
    }

    /// 전체 허용만 끈다 (시크릿별 허용은 그대로)
    pub fn unset_all_grant(&mut self) {
        self.grants.remove(ALL_KEY);
    }

    /// 전체 허용. None = 전체 허용 해제 + 개별 허용도 모두 해제
    pub fn set_all_grant(&mut self, until: Option<f64>) {
        match until {
            Some(u) => {
                self.grants.insert(ALL_KEY.to_string(), u);
            }
            None => self.grants.clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Swift 판 JSONEncoder 가 만든 모양 (키 순서 무관, notes 없는 예전 금고 포함)
    const SWIFT_VAULT: &str = r#"{"grants":{"GITHUB_TOKEN":0,"*":1794736046.25},"secrets":{"GITHUB_TOKEN":"ghp_x","aaa":"v"}}"#;

    #[test]
    fn reads_swift_vault_without_notes() {
        let v = Vault::from_json(SWIFT_VAULT.as_bytes()).unwrap();
        assert_eq!(v.names(), vec!["GITHUB_TOKEN", "aaa"]);
        assert_eq!(v.grants.get("*"), Some(&1794736046.25));
        assert!(v.notes.is_empty());
    }

    #[test]
    fn keeps_unknown_fields() {
        let v = Vault::from_json(br#"{"secrets":{},"future":{"x":1}}"#).unwrap();
        let back: serde_json::Value = serde_json::from_slice(&v.to_json()).unwrap();
        assert_eq!(back["future"]["x"], 1);
    }

    #[test]
    fn names_follow_env_var_rules() {
        assert!(is_valid_name("OPENAI_API_KEY"));
        assert!(is_valid_name("_x1"));
        assert!(!is_valid_name("1ABC"));
        assert!(!is_valid_name("A-B"));
        assert!(!is_valid_name("*"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name(&"A".repeat(129)));
        assert!(is_valid_name(&"A".repeat(128)));
    }

    #[test]
    fn rename_moves_note_and_grant() {
        let mut v = Vault::default();
        v.set("OLD", "1").unwrap();
        v.set_note("OLD", Some("memo")).unwrap();
        v.set_grant("OLD", Some(0.0)).unwrap();
        v.rename("OLD", "NEW").unwrap();
        assert!(!v.exists("OLD"));
        assert_eq!(v.get("NEW").unwrap(), "1");
        assert_eq!(v.notes.get("NEW").map(String::as_str), Some("memo"));
        assert_eq!(v.grants.get("NEW"), Some(&0.0));
    }

    #[test]
    fn rename_refuses_existing_target() {
        let mut v = Vault::default();
        v.set("A", "1").unwrap();
        v.set("B", "2").unwrap();
        assert!(matches!(v.rename("A", "B"), Err(Error::AlreadyExists(_))));
    }

    #[test]
    fn update_keeps_value_when_none_and_clears_empty_note() {
        let mut v = Vault::default();
        v.set("A", "1").unwrap();
        v.set_note("A", Some("x")).unwrap();
        v.update("A", "A", None, Some("  ")).unwrap();
        assert_eq!(v.get("A").unwrap(), "1");
        assert!(v.notes.get("A").is_none());
        v.update("A", "A", Some("2"), None).unwrap();
        assert_eq!(v.get("A").unwrap(), "2");
    }

    #[test]
    fn remove_clears_grant_and_note() {
        let mut v = Vault::default();
        v.set("A", "1").unwrap();
        v.set_note("A", Some("x")).unwrap();
        v.set_grant("A", Some(0.0)).unwrap();
        v.remove("A").unwrap();
        assert!(v.grants.is_empty() && v.notes.is_empty());
        assert!(matches!(v.remove("A"), Err(Error::NotFound(_))));
    }

    #[test]
    fn notes_are_flattened_and_limited() {
        assert_eq!(clean_note(" a\nb\tc "), "a b c");
        assert_eq!(clean_note(&"x".repeat(300)).chars().count(), 200);
    }

    #[test]
    fn unset_all_keeps_individual_grants() {
        let mut v = Vault::default();
        v.set_grant("A", Some(0.0)).unwrap();
        v.set_all_grant(Some(0.0));
        v.unset_all_grant();
        assert_eq!(v.grants.len(), 1);
        assert_eq!(v.grants.get("A"), Some(&0.0));
    }

    #[test]
    fn deny_all_clears_individual_grants() {
        let mut v = Vault::default();
        v.set_grant("A", Some(0.0)).unwrap();
        v.set_all_grant(Some(0.0));
        v.set_all_grant(None);
        assert!(v.grants.is_empty());
    }
}
