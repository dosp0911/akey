use crate::i18n::tr;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", tr(&format!("잘못된 이름: '{0}' (영문/숫자/_ 만 허용, 숫자로 시작 불가)", .0), &format!("Invalid name: '{0}' (letters, digits and _ only, not starting with a digit)", .0)))]
    InvalidName(String),
    #[error("{}", tr(&format!("'{0}' 없음", .0), &format!("'{0}' not found", .0)))]
    NotFound(String),
    #[error("{}", tr(&format!("'{0}' 이(가) 이미 있습니다", .0), &format!("'{0}' already exists", .0)))]
    AlreadyExists(String),
    /// 값이 잠겨 있어 먼저 인증해야 함 (Windows Hello 보호 값)
    #[error("{}", tr("잠겨 있습니다. 먼저 인증하세요", "Locked. Authenticate first"))]
    Locked,
    #[error("{}", tr(&format!("금고를 읽을 수 없습니다: {0}", .0), &format!("Cannot read the vault: {0}", .0)))]
    Corrupt(String),
    /// 키체인·DPAPI·Hello 등 플랫폼 저장소 오류
    #[error("{0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, Error>;
