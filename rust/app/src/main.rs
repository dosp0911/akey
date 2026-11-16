//! akey 메뉴바(트레이) 앱.
//! macOS: 인자가 있으면 CLI 로 동작한다 (같은 실행 파일 = 같은 서명 → 키체인 접근 권한이 같음).
//! Windows: 트레이 앱(akey-app.exe)만. CLI 는 별도 콘솔 실행 파일 akey.exe.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod commands;
mod platform;
mod session;
mod tray;

fn main() {
    platform::init();

    #[cfg(target_os = "macos")]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let launched_by_system = args
            .first()
            .map(|a| a.starts_with("-psn") || a.starts_with("-NS") || a.starts_with("-Apple"))
            .unwrap_or(true);
        if !launched_by_system {
            let store = platform::open_store().unwrap_or_else(|e| {
                eprintln!("akey: {e}");
                std::process::exit(1)
            });
            std::process::exit(akey_cli::run(&args, store.as_ref()));
        }
    }

    app::run();
}
