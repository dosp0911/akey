//! 독립 실행 CLI (Windows 의 akey.exe). macOS 에서는 앱 실행 파일이 CLI 를 겸하며, 이 바이너리는 개발·테스트용.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    #[cfg(target_os = "macos")]
    let code = {
        akey_macos::init_system_language();
        akey_macos::import_legacy_settings();
        akey_cli::run(&args, &akey_macos::KeychainStore::new())
    };

    #[cfg(windows)]
    let code = {
        akey_windows::init_system_language();
        match akey_windows::FileStore::open() {
            Ok(store) => akey_cli::run(&args, &store),
            Err(e) => {
                eprintln!("akey: {e}");
                1
            }
        }
    };

    #[cfg(not(any(target_os = "macos", windows)))]
    let code = {
        let _ = args;
        eprintln!("akey: unsupported platform");
        1
    };

    std::process::exit(code);
}
