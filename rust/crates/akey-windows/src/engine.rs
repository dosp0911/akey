//! Windows 금고 로직 (운영체제와 무관하게 동작해 macOS 에서도 테스트한다).
//!
//! 파일: `%LOCALAPPDATA%\akey\vault.json`
//! ```text
//! { "v": 1,
//!   "hello": { "challenge", "wrapped_sk", "pk" },     // Hello 키 준비 정보 (없으면 아직 안 만듦)
//!   "meta": base64(DPAPI(JSON{ names: {이름: {id, note}}, grants })),
//!   "items": { id: { "class": "hello" | "dpapi", "blob": base64 } } }
//! ```
//! - `hello` 값: X25519 공개키로 봉인. 저장할 때는 묻지 않고, 열 때 개인키가 필요하다.
//!   개인키는 Windows Hello 서명에서 만든 키(KEK)로 암호화되어 있어 Hello 확인 없이는 풀 수 없다.
//! - `dpapi` 값: 허용된 시크릿. 사용자 계정 암호화(DPAPI)만 쓴다 (허용 = 인증 없이 읽음).
//! - 이름·메모·허용 정보도 DPAPI 로 암호화한다.
//! - 항목 id 는 무작위라 이름을 바꿔도 다시 암호화하지 않는다.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use akey_core::grants::{self, now, Grants};
use akey_core::vault::{check_name, clean_note};
use akey_core::{tr, Error, Result, SecretStore, ALL_KEY};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

/// 운영체제 기능. Windows 에서는 DPAPI 와 Windows Hello, 테스트에서는 가짜 구현.
pub trait Platform: Send + Sync {
    fn dpapi_protect(&self, data: &[u8], entropy: &[u8]) -> Result<Vec<u8>>;
    fn dpapi_unprotect(&self, data: &[u8], entropy: &[u8]) -> Result<Vec<u8>>;
    /// Hello 키로 challenge 에 서명한다 (사용자 확인 창이 뜬다). 키가 없고 create 면 만든다.
    /// Ok(None) = 사용자가 취소
    fn hello_sign(&self, challenge: &[u8], create: bool) -> Result<Option<Vec<u8>>>;
    /// 서명이 매번 같은 값(PKCS#1 v1.5)인지 공개키로 검증한다
    fn hello_verify(&self, challenge: &[u8], signature: &[u8]) -> Result<bool>;
    /// Hello 창은 이유 문구를 받지 않으므로, 무엇을 승인하는지 따로 알린다 (Windows: 콘솔에 출력)
    fn announce(&self, _reason: &str) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Class {
    Hello,
    Dpapi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ItemBlob {
    class: Class,
    blob: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelloSetup {
    challenge: String,
    wrapped_sk: String,
    pk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    v: u32,
    #[serde(default)]
    hello: Option<HelloSetup>,
    #[serde(default)]
    meta: String,
    #[serde(default)]
    items: BTreeMap<String, ItemBlob>,
}

impl Default for VaultFile {
    fn default() -> Self {
        VaultFile { v: 1, hello: None, meta: String::new(), items: BTreeMap::new() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Entry {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Meta {
    #[serde(default)]
    names: BTreeMap<String, Entry>,
    #[serde(default)]
    grants: Grants,
}

const META_ENTROPY: &[u8] = b"akey:meta";
const SK_AAD: &[u8] = b"akey/hello-sk/v1";
const KEK_INFO: &[u8] = b"akey/hello-kek/v1";
const ITEM_INFO: &[u8] = b"akey/item/v1";

fn backend(msg: impl std::fmt::Display) -> Error {
    Error::Backend(msg.to_string())
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).map_err(backend)?;
    Ok(buf)
}

fn new_id() -> Result<String> {
    Ok(random::<16>()?.iter().map(|b| format!("{b:02x}")).collect())
}

fn b64d(s: &str) -> Result<Vec<u8>> {
    B64.decode(s).map_err(|e| Error::Corrupt(e.to_string()))
}

fn item_entropy(id: &str) -> Vec<u8> {
    format!("akey:item:{id}").into_bytes()
}

// ---- 암호 ----

fn hkdf32(ikm: &[u8], salt: &[u8], info: &[u8]) -> Zeroizing<[u8; 32]> {
    let mut out = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(Some(salt), ikm).expand(info, out.as_mut()).expect("hkdf length");
    out
}

fn aes_encrypt(key: &[u8; 32], plain: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(backend)?;
    let nonce = random::<12>()?;
    let mut out = nonce.to_vec();
    out.extend(cipher.encrypt(Nonce::from_slice(&nonce), Payload { msg: plain, aad }).map_err(backend)?);
    Ok(out)
}

fn aes_decrypt(key: &[u8; 32], data: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if data.len() < 12 {
        return Err(Error::Corrupt("short ciphertext".into()));
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(backend)?;
    let plain = cipher
        .decrypt(Nonce::from_slice(&data[..12]), Payload { msg: &data[12..], aad })
        .map_err(|_| Error::Corrupt("decrypt failed".into()))?;
    Ok(Zeroizing::new(plain))
}

/// 공개키로 봉인: 임시 X25519 키 + HKDF + AES-GCM. 결과 = 임시공개키(32) || nonce || 암호문
fn seal(pk: &[u8; 32], id: &str, plain: &[u8]) -> Result<Vec<u8>> {
    let ephemeral = StaticSecret::from(random::<32>()?);
    let epk = PublicKey::from(&ephemeral);
    let shared = ephemeral.diffie_hellman(&PublicKey::from(*pk));
    let mut salt = epk.as_bytes().to_vec();
    salt.extend_from_slice(pk);
    let key = hkdf32(shared.as_bytes(), &salt, ITEM_INFO);
    let mut out = epk.as_bytes().to_vec();
    out.extend(aes_encrypt(&key, plain, id.as_bytes())?);
    Ok(out)
}

fn open(sk: &[u8; 32], id: &str, data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if data.len() < 32 {
        return Err(Error::Corrupt("short sealed box".into()));
    }
    let secret = StaticSecret::from(*sk);
    let pk = PublicKey::from(&secret);
    let mut epk = [0u8; 32];
    epk.copy_from_slice(&data[..32]);
    let shared = secret.diffie_hellman(&PublicKey::from(epk));
    let mut salt = epk.to_vec();
    salt.extend_from_slice(pk.as_bytes());
    let key = hkdf32(shared.as_bytes(), &salt, ITEM_INFO);
    aes_decrypt(&key, &data[32..], id.as_bytes())
}

fn to32(v: &[u8]) -> Result<[u8; 32]> {
    v.try_into().map_err(|_| Error::Corrupt("bad key length".into()))
}

// ---- 저장소 ----

pub struct Engine<P: Platform> {
    platform: P,
    path: PathBuf,
    /// Hello 로 푼 개인키. lock() 까지 메모리에만 둔다.
    sk: Mutex<Option<Zeroizing<[u8; 32]>>>,
}

struct Loaded {
    file: VaultFile,
    meta: Meta,
}

impl<P: Platform> Engine<P> {
    pub fn new(platform: P, path: PathBuf) -> Self {
        Engine { platform, path, sk: Mutex::new(None) }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn cached_sk(&self) -> Option<Zeroizing<[u8; 32]>> {
        self.sk.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set_sk(&self, sk: Option<Zeroizing<[u8; 32]>>) {
        *self.sk.lock().unwrap_or_else(|e| e.into_inner()) = sk;
    }

    /// 다른 프로세스(CLI·앱)와 동시에 쓰지 않도록 잠금 파일을 잡고 실행한다.
    /// f 가 true 를 돌려주면 저장한다.
    fn with_vault<T>(&self, f: impl FnOnce(&mut Loaded) -> Result<(T, bool)>) -> Result<T> {
        let dir = self.path.parent().map(Path::to_path_buf).unwrap_or_default();
        std::fs::create_dir_all(&dir).map_err(backend)?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join("vault.lock"))
            .map_err(backend)?;
        lock.lock().map_err(backend)?;

        let mut loaded = self.load()?;
        let (value, dirty) = f(&mut loaded)?;
        if dirty {
            self.save(&mut loaded)?;
        }
        drop(lock);
        Ok(value)
    }

    fn read<T>(&self, f: impl FnOnce(&Loaded) -> Result<T>) -> Result<T> {
        self.with_vault(|l| Ok((f(l)?, false)))
    }

    fn load(&self) -> Result<Loaded> {
        let file: VaultFile = match std::fs::read(&self.path) {
            Ok(data) => serde_json::from_slice(&data).map_err(|e| Error::Corrupt(e.to_string()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => VaultFile::default(),
            Err(e) => return Err(backend(e)),
        };
        let meta = if file.meta.is_empty() {
            Meta::default()
        } else {
            let plain = self.platform.dpapi_unprotect(&b64d(&file.meta)?, META_ENTROPY)?;
            serde_json::from_slice(&plain).map_err(|e| Error::Corrupt(e.to_string()))?
        };
        Ok(Loaded { file, meta })
    }

    fn save(&self, l: &mut Loaded) -> Result<()> {
        let meta_json = Zeroizing::new(serde_json::to_vec(&l.meta).map_err(backend)?);
        l.file.meta = B64.encode(self.platform.dpapi_protect(&meta_json, META_ENTROPY)?);
        let data = serde_json::to_vec_pretty(&l.file).map_err(backend)?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, data).map_err(backend)?;
        // Windows 에서 std::fs::rename 은 기존 파일을 바꿔 쓴다 (MoveFileExW REPLACE_EXISTING)
        std::fs::rename(&tmp, &self.path).map_err(backend)
    }

    // ---- Hello 키 ----

    /// Hello 키가 없으면 만든다 (확인 창). 서명이 매번 같은 값이 아니면 쓰지 않는다.
    /// remember: 만든 직후 개인키를 기억할지. 사용자가 '인증'을 요청한 경우(authenticate)만 true.
    /// 저장처럼 인증이 필요 없는 작업 중에 만든 경우에는 개인키를 메모리에 남기지 않는다.
    fn ensure_hello(&self, file: &mut VaultFile, remember: bool) -> Result<[u8; 32]> {
        if let Some(h) = &file.hello {
            return to32(&b64d(&h.pk)?);
        }
        let challenge = random::<32>()?;
        let Some(sig) = self.platform.hello_sign(&challenge, true)? else {
            return Err(Error::Backend(tr("Windows Hello 확인이 취소되었습니다", "Windows Hello was cancelled")));
        };
        if !self.platform.hello_verify(&challenge, &sig)? {
            return Err(Error::Backend(tr(
                "이 PC 의 Windows Hello 서명 방식으로는 키를 보호할 수 없습니다",
                "This PC's Windows Hello signature format can't be used to protect keys",
            )));
        }
        let kek = hkdf32(&sig, &challenge, KEK_INFO);
        let sk = Zeroizing::new(random::<32>()?);
        let pk = PublicKey::from(&StaticSecret::from(*sk));
        file.hello = Some(HelloSetup {
            challenge: B64.encode(challenge),
            wrapped_sk: B64.encode(aes_encrypt(&kek, sk.as_ref(), SK_AAD)?),
            pk: B64.encode(pk.as_bytes()),
        });
        if remember {
            self.set_sk(Some(sk));
        }
        Ok(*pk.as_bytes())
    }

    fn unlock_with_hello(&self, setup: &HelloSetup) -> Result<bool> {
        let challenge = b64d(&setup.challenge)?;
        let Some(sig) = self.platform.hello_sign(&challenge, false)? else { return Ok(false) };
        let kek = hkdf32(&sig, &challenge, KEK_INFO);
        match aes_decrypt(&kek, &b64d(&setup.wrapped_sk)?, SK_AAD) {
            Ok(sk) => {
                self.set_sk(Some(Zeroizing::new(to32(&sk)?)));
                Ok(true)
            }
            // 서명이 예전과 다름 (Hello 재설정 등) → 풀 수 없음
            Err(_) => Err(Error::Backend(tr(
                "Windows Hello 키가 바뀌어 '매번 묻기' 값을 풀 수 없습니다",
                "The Windows Hello key changed; 'ask every time' values can't be decrypted",
            ))),
        }
    }

    // ---- 값 ----

    fn protect_value(&self, file: &mut VaultFile, id: &str, class: Class, value: &[u8]) -> Result<ItemBlob> {
        let blob = match class {
            Class::Dpapi => {
                // DPAPI 로 두는 값도 허용이 끝나면 Hello 로 다시 봉인해야 하므로 Hello 키를 먼저 준비한다.
                // (키가 없으면 maintain 이 다시 봉인하지 못해 DPAPI 로만 남는다)
                if file.hello.is_none() {
                    self.ensure_hello(file, false)?;
                }
                self.platform.dpapi_protect(value, &item_entropy(id))?
            }
            Class::Hello => {
                let pk = self.ensure_hello(file, false)?;
                seal(&pk, id, value)?
            }
        };
        Ok(ItemBlob { class, blob: B64.encode(blob) })
    }

    fn reveal_value(&self, id: &str, item: &ItemBlob) -> Result<Zeroizing<Vec<u8>>> {
        let data = b64d(&item.blob)?;
        match item.class {
            Class::Dpapi => Ok(Zeroizing::new(self.platform.dpapi_unprotect(&data, &item_entropy(id))?)),
            Class::Hello => {
                let sk = self.cached_sk().ok_or(Error::Locked)?;
                open(&sk, id, &data)
            }
        }
    }

    /// 허용 상태에 맞게 보호 방식을 맞춘다.
    /// 허용됨 → DPAPI (Hello 값을 풀어야 하므로 인증된 상태여야 함), 허용 안 됨 → Hello (묻지 않음)
    fn reclass_all(&self, l: &mut Loaded) -> Result<bool> {
        let t = now();
        let mut changed = false;
        let names: Vec<(String, String)> = l.meta.names.iter().map(|(n, e)| (n.clone(), e.id.clone())).collect();
        for (name, id) in names {
            let Some(item) = l.file.items.get(&id).cloned() else { continue };
            let want = if grants::is_granted(&name, &l.meta.grants, t) { Class::Dpapi } else { Class::Hello };
            if item.class == want {
                continue;
            }
            let plain = self.reveal_value(&id, &item)?;
            let blob = self.protect_value(&mut l.file, &id, want, &plain)?;
            l.file.items.insert(id, blob);
            changed = true;
        }
        Ok(changed)
    }

    fn class_for(name: &str, meta: &Meta) -> Class {
        if grants::is_granted(name, &meta.grants, now()) {
            Class::Dpapi
        } else {
            Class::Hello
        }
    }
}

impl<P: Platform> SecretStore for Engine<P> {
    fn names(&self) -> Result<Vec<String>> {
        self.read(|l| Ok(l.meta.names.keys().cloned().collect()))
    }

    fn notes(&self) -> Result<BTreeMap<String, String>> {
        self.read(|l| Ok(l.meta.names.iter().filter_map(|(n, e)| e.note.clone().map(|v| (n.clone(), v))).collect()))
    }

    fn grants(&self) -> Grants {
        self.read(|l| Ok(l.meta.grants.clone())).unwrap_or_default()
    }

    fn get(&self, name: &str) -> Result<String> {
        check_name(name)?;
        self.read(|l| {
            let entry = l.meta.names.get(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
            let item = l.file.items.get(&entry.id).ok_or_else(|| Error::Corrupt(format!("missing item {}", entry.id)))?;
            let plain = self.reveal_value(&entry.id, item)?;
            String::from_utf8(plain.to_vec()).map_err(|e| Error::Corrupt(e.to_string()))
        })
    }

    fn set(&self, name: &str, value: &str) -> Result<()> {
        check_name(name)?;
        self.with_vault(|l| {
            let id = match l.meta.names.get(name) {
                Some(e) => e.id.clone(),
                None => new_id()?,
            };
            let class = Self::class_for(name, &l.meta);
            let blob = self.protect_value(&mut l.file, &id, class, value.as_bytes())?;
            l.file.items.insert(id.clone(), blob);
            l.meta.names.entry(name.to_string()).or_insert(Entry { id, note: None });
            Ok(((), true))
        })
    }

    fn remove(&self, name: &str) -> Result<()> {
        check_name(name)?;
        self.with_vault(|l| {
            let entry = l.meta.names.remove(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
            l.file.items.remove(&entry.id);
            l.meta.grants.remove(name);
            Ok(((), true))
        })
    }

    fn update(&self, old: &str, new: &str, value: Option<&str>, note: Option<&str>) -> Result<()> {
        check_name(old)?;
        check_name(new)?;
        self.with_vault(|l| {
            let mut entry = l.meta.names.get(old).cloned().ok_or_else(|| Error::NotFound(old.to_string()))?;
            if new != old {
                if l.meta.names.contains_key(new) {
                    return Err(Error::AlreadyExists(new.to_string()));
                }
                l.meta.names.remove(old);
                if let Some(g) = l.meta.grants.remove(old) {
                    l.meta.grants.insert(new.to_string(), g);
                }
            }
            if let Some(note) = note {
                let cleaned = clean_note(note);
                entry.note = (!cleaned.is_empty()).then_some(cleaned);
            }
            if let Some(value) = value {
                let class = Self::class_for(new, &l.meta);
                let blob = self.protect_value(&mut l.file, &entry.id, class, value.as_bytes())?;
                l.file.items.insert(entry.id.clone(), blob);
            }
            l.meta.names.insert(new.to_string(), entry);
            // 이름이 바뀌어 허용 상태가 달라졌을 수 있다 (전체 허용 등)
            self.reclass_all(l)?;
            Ok(((), true))
        })
    }

    fn set_note(&self, name: &str, note: Option<&str>) -> Result<()> {
        check_name(name)?;
        self.with_vault(|l| {
            let entry = l.meta.names.get_mut(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
            let cleaned = clean_note(note.unwrap_or(""));
            entry.note = (!cleaned.is_empty()).then_some(cleaned);
            Ok(((), true))
        })
    }

    fn set_grant(&self, name: &str, until: Option<f64>) -> Result<()> {
        check_name(name)?;
        self.with_vault(|l| {
            match until {
                Some(u) => {
                    l.meta.grants.insert(name.to_string(), u);
                }
                None => {
                    l.meta.grants.remove(name);
                }
            }
            self.reclass_all(l)?;
            Ok(((), true))
        })
    }

    fn set_all_grant(&self, until: Option<f64>) -> Result<()> {
        self.with_vault(|l| {
            match until {
                Some(u) => {
                    l.meta.grants.insert(ALL_KEY.to_string(), u);
                }
                None => l.meta.grants.clear(),
            }
            self.reclass_all(l)?;
            Ok(((), true))
        })
    }

    fn unset_all_grant(&self) -> Result<()> {
        // 허용이 줄기만 하므로 DPAPI → Hello 봉인만 일어나 인증이 필요 없다
        self.with_vault(|l| {
            l.meta.grants.remove(ALL_KEY);
            self.reclass_all(l)?;
            Ok(((), true))
        })
    }

    fn authenticate(&self, reason: &str) -> bool {
        // Windows Hello 창은 이유 문구를 받지 않아 무엇을 승인하는지 먼저 알린다.
        // 확인 결과로 개인키를 풀어 둔다.
        self.platform.announce(reason);
        let result = self.with_vault(|l| match l.file.hello.clone() {
            Some(setup) => Ok((self.unlock_with_hello(&setup)?, false)),
            None => {
                // 처음: Hello 키를 만들며 확인 (사용자가 인증을 요청했으니 개인키를 기억함)
                self.ensure_hello(&mut l.file, true)?;
                Ok((true, true))
            }
        });
        result.unwrap_or(false)
    }

    fn lock(&self) {
        self.set_sk(None);
    }

    /// 만료된 허용의 값을 다시 Hello 로 봉인한다 (묻지 않음)
    fn maintain(&self) {
        let _ = self.with_vault(|l| {
            let t = now();
            let expired: Vec<String> = l
                .meta
                .names
                .iter()
                .filter(|(n, e)| {
                    !grants::is_granted(n, &l.meta.grants, t)
                        && l.file.items.get(&e.id).map(|i| i.class == Class::Dpapi).unwrap_or(false)
                })
                .map(|(n, _)| n.clone())
                .collect();
            if expired.is_empty() || l.file.hello.is_none() {
                return Ok(((), false));
            }
            Ok(((), self.reclass_all(l)?))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    /// 가짜 DPAPI(바이트 뒤집기 + 표시) 와 가짜 Hello(고정 키로 HMAC 비슷한 결정적 서명)
    #[derive(Clone, Default)]
    struct Fake {
        cancel: Arc<AtomicBool>,
        signs: Arc<AtomicUsize>,
        announced: Arc<std::sync::Mutex<Vec<String>>>,
        hello_key: [u8; 32],
    }

    impl Platform for Fake {
        fn dpapi_protect(&self, data: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
            let mut out = b"DPAPI".to_vec();
            out.extend(entropy.iter().chain(data.iter()).rev());
            Ok(out)
        }
        fn dpapi_unprotect(&self, data: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
            let body: Vec<u8> = data[5..].iter().rev().copied().collect();
            assert!(body.starts_with(entropy), "entropy mismatch");
            Ok(body[entropy.len()..].to_vec())
        }
        fn hello_sign(&self, challenge: &[u8], _create: bool) -> Result<Option<Vec<u8>>> {
            self.signs.fetch_add(1, Ordering::SeqCst);
            if self.cancel.load(Ordering::SeqCst) {
                return Ok(None);
            }
            Ok(Some(hkdf32(&self.hello_key, challenge, b"fake-sign").to_vec()))
        }
        fn hello_verify(&self, _challenge: &[u8], _signature: &[u8]) -> Result<bool> {
            Ok(true)
        }
        fn announce(&self, reason: &str) {
            self.announced.lock().unwrap().push(reason.to_string());
        }
    }

    fn engine(name: &str) -> (Engine<Fake>, Fake) {
        let dir = std::env::temp_dir().join(format!("akey-win-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let fake = Fake { hello_key: [7u8; 32], ..Default::default() };
        (Engine::new(fake.clone(), dir.join("vault.json")), fake)
    }

    fn raw(e: &Engine<Fake>) -> String {
        String::from_utf8(std::fs::read(e.path()).unwrap()).unwrap()
    }

    #[test]
    fn ask_values_need_hello_and_never_hit_disk_in_plain() {
        let (e, fake) = engine("ask");
        e.set("API_KEY", "super-secret-value").unwrap(); // 처음: Hello 키 생성
        assert_eq!(fake.signs.load(Ordering::SeqCst), 1);
        assert!(!raw(&e).contains("super-secret-value"));
        assert!(!raw(&e).contains("API_KEY"), "이름도 DPAPI 로 감싸져 있어야 함");

        e.lock();
        assert!(matches!(e.get("API_KEY"), Err(Error::Locked)));
        assert!(e.authenticate("read"));
        assert_eq!(e.get("API_KEY").unwrap(), "super-secret-value");

        // 새 프로세스(=새 Engine)는 기억한 키가 없다
        let e2 = Engine::new(fake.clone(), e.path().to_path_buf());
        assert!(matches!(e2.get("API_KEY"), Err(Error::Locked)));
        fake.cancel.store(true, Ordering::SeqCst);
        assert!(!e2.authenticate("read"), "취소하면 실패");
    }

    #[test]
    fn allow_moves_to_dpapi_and_deny_moves_back_without_prompt() {
        let (e, fake) = engine("allow");
        e.set("TOKEN", "v1").unwrap();
        e.lock();
        // 인증 없이 허용하려 하면 값을 풀 수 없어 거부
        assert!(matches!(e.set_grant("TOKEN", Some(0.0)), Err(Error::Locked)));
        assert!(e.authenticate("allow"));
        e.set_grant("TOKEN", Some(0.0)).unwrap();
        e.lock();
        // 허용된 값은 Hello 없이 읽힌다
        let fresh = Engine::new(fake.clone(), e.path().to_path_buf());
        assert_eq!(fresh.get("TOKEN").unwrap(), "v1");
        // 해제는 묻지 않고 다시 Hello 로 봉인
        let signs = fake.signs.load(Ordering::SeqCst);
        fresh.set_grant("TOKEN", None).unwrap();
        assert_eq!(fake.signs.load(Ordering::SeqCst), signs);
        assert!(matches!(fresh.get("TOKEN"), Err(Error::Locked)));
    }

    #[test]
    fn allow_all_includes_future_and_expiry_reseals() {
        let (e, fake) = engine("all");
        e.set("A", "a").unwrap();
        assert!(matches!(e.set_all_grant(Some(now() + 3600.0)), Err(Error::Locked)), "저장할 때 만든 키는 기억하지 않음");
        assert!(e.authenticate("allow all"));
        e.set_all_grant(Some(now() + 3600.0)).unwrap();
        e.set("B", "b").unwrap(); // 전체 허용 중이라 바로 DPAPI
        e.lock();
        let fresh = Engine::new(fake.clone(), e.path().to_path_buf());
        assert_eq!(fresh.get("A").unwrap(), "a");
        assert_eq!(fresh.get("B").unwrap(), "b");

        // 만료를 흉내: 과거 시각으로 바꾸고 maintain
        fresh.with_vault(|l| {
            l.meta.grants.insert(ALL_KEY.to_string(), now() - 10.0);
            Ok(((), true))
        })
        .unwrap();
        fresh.maintain();
        assert!(matches!(fresh.get("A"), Err(Error::Locked)));
        assert!(matches!(fresh.get("B"), Err(Error::Locked)));
    }

    #[test]
    fn unset_all_keeps_individual_and_reseals_others_without_prompt() {
        let (e, fake) = engine("unset-all");
        e.set("KEEP", "k").unwrap();
        e.set("OTHER", "o").unwrap();
        assert!(e.authenticate("allow"));
        e.set_grant("KEEP", Some(0.0)).unwrap();
        e.set_all_grant(Some(0.0)).unwrap();
        e.lock();
        let signs = fake.signs.load(Ordering::SeqCst);
        let fresh = Engine::new(fake.clone(), e.path().to_path_buf());
        fresh.unset_all_grant().unwrap();
        assert_eq!(fake.signs.load(Ordering::SeqCst), signs, "끄는 데는 Hello 가 필요 없음");
        assert_eq!(fresh.get("KEEP").unwrap(), "k", "시크릿별 허용은 남음");
        assert!(matches!(fresh.get("OTHER"), Err(Error::Locked)), "나머지는 다시 Hello 로 봉인");
    }

    #[test]
    fn dpapi_values_created_before_any_hello_key_still_get_resealed() {
        // 전체 허용을 먼저 켜고(값 없음) 첫 시크릿을 넣으면 DPAPI 로 저장된다.
        // 이때도 Hello 키를 준비해 두어야 만료 뒤 다시 봉인된다.
        let (e, fake) = engine("dpapi-first");
        e.set_all_grant(Some(now() + 3600.0)).unwrap();
        e.set("FIRST", "1").unwrap();
        assert_eq!(fake.signs.load(Ordering::SeqCst), 1, "첫 DPAPI 저장 전에 Hello 키를 만듦");
        e.with_vault(|l| {
            l.meta.grants.insert(ALL_KEY.to_string(), now() - 10.0);
            Ok(((), true))
        })
        .unwrap();
        let fresh = Engine::new(fake.clone(), e.path().to_path_buf());
        fresh.maintain();
        assert!(matches!(fresh.get("FIRST"), Err(Error::Locked)), "만료 뒤 Hello 로 다시 봉인");
    }

    #[test]
    fn announce_is_called_before_hello_prompt() {
        let (e, fake) = engine("announce");
        e.set("K", "1").unwrap();
        e.lock();
        assert!(e.authenticate("read secrets: K"));
        assert_eq!(fake.announced.lock().unwrap().as_slice(), ["read secrets: K"]);
    }

    #[test]
    fn rename_keeps_value_note_and_grant_without_reencrypting() {
        let (e, _) = engine("rename");
        e.set("OLD", "value").unwrap();
        e.set_note("OLD", Some("memo")).unwrap();
        let blob_before: serde_json::Value = serde_json::from_str(&raw(&e)).unwrap();
        e.rename("OLD", "NEW").unwrap();
        let blob_after: serde_json::Value = serde_json::from_str(&raw(&e)).unwrap();
        assert_eq!(blob_before["items"], blob_after["items"], "값은 다시 암호화하지 않음");
        assert_eq!(e.names().unwrap(), vec!["NEW"]);
        assert_eq!(e.notes().unwrap().get("NEW").map(String::as_str), Some("memo"));
        assert!(e.authenticate("read"));
        assert_eq!(e.get("NEW").unwrap(), "value");
        e.set("X", "x").unwrap();
        assert!(matches!(e.rename("NEW", "X"), Err(Error::AlreadyExists(_))));
    }

    #[test]
    fn remove_and_update_value() {
        let (e, _) = engine("update");
        e.set("K", "1").unwrap();
        e.update("K", "K", Some("2"), Some("n")).unwrap();
        assert!(e.authenticate("read"));
        assert_eq!(e.get("K").unwrap(), "2");
        e.remove("K").unwrap();
        assert!(e.names().unwrap().is_empty());
        assert!(matches!(e.remove("K"), Err(Error::NotFound(_))));
    }

    #[test]
    fn changed_hello_key_is_detected() {
        let (e, fake) = engine("changed");
        e.set("K", "1").unwrap();
        e.lock();
        let other = Fake { hello_key: [9u8; 32], ..fake };
        let e2 = Engine::new(other, e.path().to_path_buf());
        assert!(!e2.authenticate("read"));
    }

    #[test]
    fn sealed_box_roundtrip_and_binding() {
        let sk = [3u8; 32];
        let pk = *PublicKey::from(&StaticSecret::from(sk)).as_bytes();
        let sealed = seal(&pk, "id1", b"hello").unwrap();
        assert_eq!(open(&sk, "id1", &sealed).unwrap().as_slice(), b"hello");
        assert!(open(&sk, "id2", &sealed).is_err(), "다른 항목 id 로는 열 수 없음");
    }
}
