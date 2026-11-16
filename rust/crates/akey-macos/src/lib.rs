//! macOS 저장소: 로그인 키체인의 항목 하나(service `akey`, account `vault`)에 금고 JSON 을 둔다.
//! Swift 판과 같은 항목이라 데이터를 옮길 필요가 없다.
//!
//! 키체인 항목을 하나만 쓰는 이유: Apple 개발자 팀 없이 서명한 앱은 항목이 빌드 지문(cdhash)에 묶여,
//! 다시 빌드하면 항목마다 키체인 비밀번호를 묻는다. 하나면 한 번만 묻는다.
#![cfg(target_os = "macos")]

pub mod status;

use std::collections::BTreeMap;
use std::sync::mpsc;

use akey_core::{tr, Error, Grants, Result, SecretStore, Settings, Vault};
use block2::RcBlock;
use objc2::runtime::Bool;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSError, NSLocale, NSString};
use objc2_local_authentication::{LAContext, LAPolicy};
use security_framework::passwords::{generic_password, set_generic_password_options, PasswordOptions};

const ERR_ITEM_NOT_FOUND: i32 = -25300;

pub struct KeychainStore {
    service: String,
    account: String,
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeychainStore {
    /// 개발(debug) 빌드에서만 `AKEY_KEYCHAIN_SERVICE` 로 다른 항목을 쓰게 할 수 있다 (실제 금고를 건드리지 않는 테스트용).
    /// 배포 빌드에서는 환경변수로 금고를 바꿀 수 없다 (가짜 금고를 가리키게 하는 공격 방지).
    pub fn new() -> Self {
        #[cfg(debug_assertions)]
        let service = std::env::var("AKEY_KEYCHAIN_SERVICE").unwrap_or_else(|_| "akey".to_string());
        #[cfg(not(debug_assertions))]
        let service = "akey".to_string();
        KeychainStore { service, account: "vault".to_string() }
    }

    fn options(&self) -> PasswordOptions {
        PasswordOptions::new_generic_password(&self.service, &self.account)
    }

    fn load(&self) -> Result<Vault> {
        match generic_password(self.options()) {
            Ok(data) => Vault::from_json(&data),
            Err(e) if e.code() == ERR_ITEM_NOT_FOUND => Ok(Vault::default()),
            Err(e) => Err(keychain_error(e)),
        }
    }

    fn save(&self, vault: &Vault) -> Result<()> {
        let mut options = self.options();
        options.set_label(&format!("akey: {}", self.account));
        set_generic_password_options(&vault.to_json(), options).map_err(keychain_error)
    }

    /// 읽기→바꾸기→쓰기를 잠금 파일로 감싼다. 앱과 CLI(에이전트)가 동시에 쓰면 한쪽 변경이
    /// 사라질 수 있다 (예: 사용자가 허용을 해제한 직후 CLI 가 예전 내용으로 덮어써 허용이 되살아남).
    fn modify(&self, f: impl FnOnce(&mut Vault) -> Result<()>) -> Result<()> {
        let _lock = self.write_lock()?;
        let mut vault = self.load()?;
        f(&mut vault)?;
        self.save(&vault)
    }

    /// `~/Library/Application Support/akey/vault.lock` 를 독점으로 잡는다 (돌려준 파일이 닫힐 때 풀림)
    fn write_lock(&self) -> Result<std::fs::File> {
        let dir = Settings::dir();
        std::fs::create_dir_all(&dir).map_err(|e| Error::Backend(e.to_string()))?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(format!("{}.lock", self.service)))
            .map_err(|e| Error::Backend(e.to_string()))?;
        file.lock().map_err(|e| Error::Backend(e.to_string()))?;
        Ok(file)
    }
}

fn keychain_error(e: security_framework::base::Error) -> Error {
    let code = e.code();
    Error::Backend(tr(&format!("키체인 오류 {code}: {e}"), &format!("Keychain error {code}: {e}")))
}

impl SecretStore for KeychainStore {
    fn names(&self) -> Result<Vec<String>> {
        Ok(self.load()?.names())
    }

    fn notes(&self) -> Result<BTreeMap<String, String>> {
        Ok(self.load()?.notes)
    }

    fn grants(&self) -> Grants {
        self.load().map(|v| v.grants).unwrap_or_default()
    }

    fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.load()?.exists(name))
    }

    fn get(&self, name: &str) -> Result<String> {
        self.load()?.get(name)
    }

    fn set(&self, name: &str, value: &str) -> Result<()> {
        self.modify(|v| v.set(name, value))
    }

    fn remove(&self, name: &str) -> Result<()> {
        self.modify(|v| v.remove(name))
    }

    fn update(&self, old: &str, new: &str, value: Option<&str>, note: Option<&str>) -> Result<()> {
        self.modify(|v| v.update(old, new, value, note))
    }

    fn set_note(&self, name: &str, note: Option<&str>) -> Result<()> {
        self.modify(|v| v.set_note(name, note))
    }

    fn set_grant(&self, name: &str, until: Option<f64>) -> Result<()> {
        self.modify(|v| v.set_grant(name, until))
    }

    fn set_all_grant(&self, until: Option<f64>) -> Result<()> {
        self.modify(|v| {
            v.set_all_grant(until);
            Ok(())
        })
    }

    fn unset_all_grant(&self) -> Result<()> {
        self.modify(|v| {
            v.unset_all_grant();
            Ok(())
        })
    }

    fn authenticate(&self, reason: &str) -> bool {
        authenticate(reason)
    }
}

/// Touch ID → 실패/미지원 시 Mac 로그인 비밀번호. 쓸 수 없는 환경이면 false (fail closed).
/// 응답은 시스템의 다른 큐에서 오므로 호출한 스레드를 막고 기다려도 된다 (메인 스레드에서 부르지 말 것).
pub fn authenticate(reason: &str) -> bool {
    let policy = LAPolicy::DeviceOwnerAuthentication;
    let context = unsafe { LAContext::new() };
    if unsafe { context.canEvaluatePolicy_error(policy) }.is_err() {
        return false;
    }
    let (tx, rx) = mpsc::channel::<bool>();
    let reply = RcBlock::new(move |ok: Bool, _error: *mut NSError| {
        let _ = tx.send(ok.as_bool());
    });
    let reason = NSString::from_str(reason);
    unsafe { context.evaluatePolicy_localizedReason_reply(policy, &reason, &reply) };
    rx.recv().unwrap_or(false)
}

/// 메뉴바 전용 앱은 평소 활성 앱이 아니라서 그대로 물으면 Touch ID 없이 비밀번호 창만 뜰 수 있다.
/// 인증 직전에 메인 스레드에서 불러 앱을 앞으로 꺼낸다.
pub fn activate_app() {
    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
    }
}

/// 시스템 첫 번째 선호 언어를 akey-core 에 알려 준다 (언어 기본값 결정용)
pub fn init_system_language() {
    let languages = NSLocale::preferredLanguages();
    if let Some(first) = languages.firstObject() {
        akey_core::i18n::set_system_language(&first.to_string());
    }
}

/// Swift 판이 UserDefaults(local.akey) 에 저장한 설정을 처음 한 번 settings.json 으로 가져온다.
pub fn import_legacy_settings() {
    if Settings::exists() {
        return;
    }
    let Some(home) = std::env::var_os("HOME") else { return };
    let path = std::path::Path::new(&home).join("Library/Preferences/local.akey.plist");
    let Ok(plist::Value::Dictionary(dict)) = plist::Value::from_file(&path) else { return };
    let mut settings = Settings::default();
    if let Some(lang) = dict.get("language").and_then(|v| v.as_string()) {
        settings.language = akey_core::Lang::from_code(lang);
    }
    if let Some(minutes) = dict.get("unlockMinutes").and_then(|v| v.as_signed_integer()) {
        if akey_core::settings::UNLOCK_CHOICES.contains(&(minutes as u32)) {
            settings.unlock_minutes = minutes as u32;
        }
    }
    if let Some(conceal) = dict.get("concealClipboard").and_then(|v| v.as_boolean()) {
        settings.conceal_clipboard = conceal;
    }
    let _ = settings.save();
}
