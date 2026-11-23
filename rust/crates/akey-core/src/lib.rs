//! akey 공용 로직. 플랫폼(macOS 키체인, Windows DPAPI+Hello)은 [`SecretStore`] 를 구현한다.

pub mod dotenv;
pub mod error;
pub mod grants;
pub mod i18n;
pub mod settings;
pub mod vault;

use std::collections::BTreeMap;

pub use error::{Error, Result};
pub use grants::Grants;
pub use i18n::{tr, Lang};
pub use settings::{PopoverSize, Settings};
pub use vault::{is_valid_name, Groups, Vault, ALL_KEY, GROUP_COUNT};

/// 시크릿 저장소. CLI 와 앱은 이 인터페이스만 쓴다.
///
/// 인증 규칙(언제 묻는지)은 호출하는 쪽(CLI·앱)이 정하고, 저장소는 저장과 암호화만 맡는다.
/// 단, Windows 에서는 "매번 묻기" 값이 Hello 키로 암호화되어 있어
/// [`SecretStore::authenticate`] 가 성공해야 [`SecretStore::get`] 이 그 값을 풀 수 있다.
pub trait SecretStore: Send + Sync {
    fn names(&self) -> Result<Vec<String>>;
    fn notes(&self) -> Result<BTreeMap<String, String>>;
    /// 읽기 실패 시 빈 값 (= 허용 없음, fail closed)
    fn grants(&self) -> Grants;
    fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.names()?.iter().any(|n| n == name))
    }
    fn get(&self, name: &str) -> Result<String>;
    fn set(&self, name: &str, value: &str) -> Result<()>;
    fn remove(&self, name: &str) -> Result<()>;
    fn update(&self, old: &str, new: &str, value: Option<&str>, note: Option<&str>) -> Result<()>;
    fn rename(&self, old: &str, new: &str) -> Result<()> {
        self.update(old, new, None, None)
    }
    fn set_note(&self, name: &str, note: Option<&str>) -> Result<()>;
    fn set_grant(&self, name: &str, until: Option<f64>) -> Result<()>;
    /// 묶음(동그라미) 이름과 시크릿별 묶음 번호
    fn groups(&self) -> Result<Groups>;
    /// 시크릿을 묶음으로 옮긴다 (인증 필요 없음: 정리일 뿐 권한이 바뀌지 않는다)
    fn set_group(&self, name: &str, group: u8) -> Result<()>;
    fn set_group_name(&self, group: u8, label: &str) -> Result<()>;
    /// 전체 허용. None = 전체 허용과 시크릿별 허용을 모두 해제 (CLI `deny --all`)
    fn set_all_grant(&self, until: Option<f64>) -> Result<()>;
    /// 전체 허용만 끈다 (시크릿별 허용은 그대로)
    fn unset_all_grant(&self) -> Result<()>;

    /// Touch ID / Windows Hello (실패·미지원 시 false = fail closed).
    /// Windows 에서는 성공하면 Hello 보호 값을 풀 키를 [`SecretStore::lock`] 전까지 기억한다.
    fn authenticate(&self, reason: &str) -> bool;
    /// 기억한 키를 지운다 (macOS 는 할 일 없음)
    fn lock(&self) {}
    /// 만료된 허용 정리 등 주기적 관리 (Windows: 만료된 값을 다시 Hello 로 봉인)
    fn maintain(&self) {}
}
