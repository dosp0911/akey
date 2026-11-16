//! Windows 실제 구현: DPAPI, Windows Hello(KeyCredentialManager), 콘솔 Ctrl+C, 시스템 언어

use akey_core::{tr, Error, Result};
use windows::core::{Array, HSTRING, PCWSTR};
use windows::Security::Credentials::{KeyCredential, KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus};
use windows::Security::Cryptography::Core::{
    AsymmetricAlgorithmNames, AsymmetricKeyAlgorithmProvider, CryptographicEngine, CryptographicPublicKeyBlobType,
};
use windows::Security::Cryptography::CryptographicBuffer;
use windows::Storage::Streams::IBuffer;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};

use crate::engine::Platform;

const KEY_NAME: &str = "akey";

fn win(e: windows::core::Error) -> Error {
    Error::Backend(tr(&format!("Windows 오류: {e}"), &format!("Windows error: {e}")))
}

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
}

/// CryptProtectData 결과를 복사하고 LocalFree 로 돌려준다
unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
    let _ = LocalFree(Some(HLOCAL(out.pbData as *mut core::ffi::c_void)));
    v
}

fn buffer(data: &[u8]) -> Result<IBuffer> {
    CryptographicBuffer::CreateFromByteArray(data).map_err(win)
}

fn bytes(buf: &IBuffer) -> Result<Vec<u8>> {
    let mut arr = Array::<u8>::new();
    CryptographicBuffer::CopyToByteArray(buf, &mut arr).map_err(win)?;
    Ok(arr.to_vec())
}

pub struct WinPlatform;

impl WinPlatform {
    fn open_credential(&self, create: bool) -> Result<Option<KeyCredential>> {
        let supported = KeyCredentialManager::IsSupportedAsync().map_err(win)?.join().map_err(win)?;
        if !supported {
            return Err(Error::Backend(tr(
                "Windows Hello 가 설정되어 있지 않습니다. 설정 → 계정 → 로그인 옵션에서 PIN 이나 지문을 먼저 설정하세요",
                "Windows Hello isn't set up. Set a PIN or fingerprint in Settings → Accounts → Sign-in options first",
            )));
        }
        let name = HSTRING::from(KEY_NAME);
        let mut result = KeyCredentialManager::OpenAsync(&name).map_err(win)?.join().map_err(win)?;
        if result.Status().map_err(win)? == KeyCredentialStatus::NotFound && create {
            result = KeyCredentialManager::RequestCreateAsync(&name, KeyCredentialCreationOption::FailIfExists)
                .map_err(win)?
                .join()
                .map_err(win)?;
        }
        match result.Status().map_err(win)? {
            KeyCredentialStatus::Success => Ok(Some(result.Credential().map_err(win)?)),
            KeyCredentialStatus::UserCanceled | KeyCredentialStatus::UserPrefersPassword => Ok(None),
            KeyCredentialStatus::NotFound => Err(Error::Backend(tr(
                "Windows Hello 키가 없습니다. 'akey' 키가 지워졌다면 '매번 묻기' 값은 복구할 수 없습니다",
                "The Windows Hello key is missing. If the 'akey' key was deleted, 'ask every time' values can't be recovered",
            ))),
            other => Err(Error::Backend(tr(
                &format!("Windows Hello 오류 (상태 {})", other.0),
                &format!("Windows Hello error (status {})", other.0),
            ))),
        }
    }
}

impl Platform for WinPlatform {
    fn dpapi_protect(&self, data: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
        let input = blob(data);
        let ent = blob(entropy);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(&input, PCWSTR::null(), Some(&ent), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
                .map_err(win)?;
            Ok(take(out))
        }
    }

    fn dpapi_unprotect(&self, data: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
        let input = blob(data);
        let ent = blob(entropy);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(&input, None, Some(&ent), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out).map_err(win)?;
            Ok(take(out))
        }
    }

    fn hello_sign(&self, challenge: &[u8], create: bool) -> Result<Option<Vec<u8>>> {
        let Some(credential) = self.open_credential(create)? else { return Ok(None) };
        let result = credential.RequestSignAsync(&buffer(challenge)?).map_err(win)?.join().map_err(win)?;
        match result.Status().map_err(win)? {
            KeyCredentialStatus::Success => Ok(Some(bytes(&result.Result().map_err(win)?)?)),
            KeyCredentialStatus::UserCanceled | KeyCredentialStatus::UserPrefersPassword => Ok(None),
            other => Err(Error::Backend(tr(
                &format!("Windows Hello 서명 오류 (상태 {})", other.0),
                &format!("Windows Hello signing error (status {})", other.0),
            ))),
        }
    }

    fn announce(&self, reason: &str) {
        // 콘솔(akey.exe)에서는 Hello 창 전에 무엇을 승인하는지 보여 준다. 트레이 앱은 콘솔이 없어 무시된다.
        eprintln!("akey: {} — {reason}", tr("Windows Hello 확인", "Windows Hello confirmation"));
    }

    fn hello_verify(&self, challenge: &[u8], signature: &[u8]) -> Result<bool> {
        let Some(credential) = self.open_credential(false)? else { return Ok(false) };
        let public = credential
            .RetrievePublicKeyWithBlobType(CryptographicPublicKeyBlobType::X509SubjectPublicKeyInfo)
            .map_err(win)?;
        let algorithm = AsymmetricAlgorithmNames::RsaSignPkcs1Sha256().map_err(win)?;
        let provider = AsymmetricKeyAlgorithmProvider::OpenAlgorithm(&algorithm).map_err(win)?;
        let key = provider
            .ImportPublicKeyWithBlobType(&public, CryptographicPublicKeyBlobType::X509SubjectPublicKeyInfo)
            .map_err(win)?;
        CryptographicEngine::VerifySignature(&key, &buffer(challenge)?, &buffer(signature)?).map_err(win)
    }
}

/// exec 로 실행한 하위 프로세스가 Ctrl+C 를 받도록 부모(akey)는 무시한다
pub fn ignore_ctrl_c() {
    unsafe extern "system" fn handler(_ctrl_type: u32) -> windows::core::BOOL {
        windows::core::BOOL(1)
    }
    unsafe {
        let _ = windows::Win32::System::Console::SetConsoleCtrlHandler(Some(handler), true);
    }
}

/// 시스템 언어(예: ko-KR)를 akey-core 에 알려 준다
pub fn init_system_language() {
    let mut buf = [0u16; 85];
    let len = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buf) };
    if len > 1 {
        let name = String::from_utf16_lossy(&buf[..(len as usize - 1)]);
        akey_core::i18n::set_system_language(&name);
    }
}
