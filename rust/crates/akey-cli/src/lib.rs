//! akey CLI. Swift 판 CLI.swift 와 같은 명령·문구·종료 코드(거부 77)를 쓴다.

pub mod mask;

use std::io::{IsTerminal, Read, Write};

use akey_core::grants::{self, now};
use akey_core::{is_valid_name, tr, Error, SecretStore};

pub const EXIT_DENIED: i32 = 77;

pub fn usage() -> String {
    tr(
        "사용법:
  akey list                         이름, 허용 상태, 메모 (값은 표시 안 함)
  akey exec NAME... -- CMD...       환경변수로 주입해 CMD 실행 (에이전트 권장, 출력의 값은 ***)
  akey get NAME                     값 출력
  akey set NAME [--note 메모]       저장/수정 (값은 숨김 입력 또는 stdin)
  akey note NAME [메모]             메모 보기 / 메모 쓰기 (쓰기는 인증)
  akey note NAME --clear            메모 지우기
  akey rm NAME                      삭제
  akey mv OLD NEW                   이름 바꾸기 (값·메모·허용 유지)
  akey allow NAME [--hours N]       인증 없이 읽기 허용 (N 생략 시 무기한)
  akey allow --all [--hours N]      모든 시크릿(앞으로 추가될 것 포함) 허용
  akey deny NAME                    허용 해제 (다시 매번 묻기)
  akey deny --all                   전체 허용과 개별 허용을 모두 해제

get/exec 는 허용되지 않은 시크릿이면 Touch ID·Windows Hello(또는 로그인 비밀번호)를 묻습니다.
allow, rm, mv, 덮어쓰기, 메모 바꾸기는 항상 인증이 필요합니다.
새 시크릿 등록(set)은 인증 없이 됩니다. 에이전트는 값을 명령줄에 쓰지 말고 파이프로 넘기세요:
  printf %s \"$VALUE\" | akey set NAME
출력 언어는 메뉴바(트레이) 아이콘 오른쪽 클릭 → 설정에서 바꿉니다.",
        "Usage:
  akey list                         names, access and notes (values are never shown)
  akey exec NAME... -- CMD...       run CMD with secrets as env vars (for agents; values in output show as ***)
  akey get NAME                     print a value
  akey set NAME [--note NOTE]       save or update (hidden prompt or stdin)
  akey note NAME [NOTE]             show / write a note (writing asks)
  akey note NAME --clear            clear a note
  akey rm NAME                      delete
  akey mv OLD NEW                   rename (keeps value, note and access)
  akey allow NAME [--hours N]       let agents read without asking (no N = no expiry)
  akey allow --all [--hours N]      allow every secret, including future ones
  akey deny NAME                    revoke (ask every time again)
  akey deny --all                   revoke all-access and every per-secret access

get/exec ask for Touch ID / Windows Hello (or your login password) unless the secret is allowed.
allow, rm, mv, overwriting and changing a note always ask.
Adding a new secret (set) doesn't ask. Agents should pipe the value, never put it on the command line:
  printf %s \"$VALUE\" | akey set NAME
Change the output language from the menu bar (tray) icon: right-click → Settings.",
        "用法：
  akey list                         名称、访问状态和备注（从不显示值）
  akey exec NAME... -- CMD...       以环境变量注入后运行 CMD（推荐助手使用，输出中的值显示为 ***）
  akey get NAME                     输出值
  akey set NAME [--note 备注]       保存或更新（隐藏输入或 stdin）
  akey note NAME [备注]             查看 / 写入备注（写入需要验证）
  akey note NAME --clear            清除备注
  akey rm NAME                      删除
  akey mv OLD NEW                   重命名（保留值、备注和访问权限）
  akey allow NAME [--hours N]       允许助手无需验证即可读取（省略 N 表示不过期）
  akey allow --all [--hours N]      允许所有密钥（包括以后添加的）
  akey deny NAME                    撤销访问（恢复每次询问）
  akey deny --all                   撤销全部访问和每个密钥的访问

get/exec 读取未被允许的密钥时会要求 Touch ID / Windows Hello（或登录密码）。
allow、rm、mv、覆盖和修改备注每次都需要验证。
添加新密钥（set）无需验证。助手应通过管道传入值，不要写在命令行中：
  printf %s \"$VALUE\" | akey set NAME
输出语言可在菜单栏（托盘）图标上右键 → 设置中更改。",
    )
}

fn err(message: &str) {
    let _ = writeln!(std::io::stderr(), "akey: {message}");
}

/// args: 프로그램 이름을 뺀 인자. 반환값: 프로세스 종료 코드
pub fn run(args: &[String], store: &dyn SecretStore) -> i32 {
    store.maintain();
    match dispatch(args, store) {
        Ok(code) => code,
        Err(e) => {
            err(&e.to_string());
            1
        }
    }
}

fn bad() -> i32 {
    let _ = writeln!(std::io::stderr(), "{}", usage());
    2
}

fn denied() -> i32 {
    err(&tr("인증 실패 또는 취소됨", "Authentication failed or cancelled", "验证失败或已取消"));
    EXIT_DENIED
}

fn dispatch(args: &[String], store: &dyn SecretStore) -> Result<i32, Error> {
    let Some(command) = args.first() else { return Ok(bad()) };
    let rest: Vec<&str> = args[1..].iter().map(String::as_str).collect();
    match command.as_str() {
        "list" | "ls" => {
            let grants = store.grants();
            let t = now();
            if let Some(all) = grants::all_grant_label(&grants, t) {
                println!("{}", tr(&format!("# 전체 허용: {all}"), &format!("# All allowed: {all}"), &format!("# 全部允许：{all}")));
            }
            let notes = store.notes()?;
            for name in store.names()? {
                let mut line = format!("{name}\t{}", grants::grant_label(&name, &grants, t));
                if let Some(note) = notes.get(&name) {
                    line.push('\t');
                    line.push_str(note);
                }
                println!("{line}");
            }
            Ok(0)
        }

        "note" => {
            let Some(&name) = rest.first() else { return Ok(bad()) };
            if !store.exists(name)? {
                return Err(Error::NotFound(name.to_string()));
            }
            let text = &rest[1..];
            if text.is_empty() {
                if let Some(note) = store.notes()?.get(name) {
                    println!("{note}");
                }
                return Ok(0);
            }
            // 기존 시크릿의 메모는 사용자가 허용 여부를 판단할 때 보는 정보라, 에이전트가 몰래 바꾸지
            // 못하게 인증을 받는다 (새 시크릿을 등록하며 쓰는 `set --note` 는 인증 없음)
            if !store.authenticate(&tr(&format!("'{name}' 시크릿의 메모를 바꾸려고 합니다"), &format!("change the note of '{name}'"), &format!("修改 '{name}' 的备注"))) {
                return Ok(denied());
            }
            if text == ["--clear"] {
                store.set_note(name, None)?;
                err(&tr(&format!("메모 지움: {name}"), &format!("Note cleared: {name}"), &format!("已清除备注：{name}")));
            } else {
                store.set_note(name, Some(&text.join(" ")))?;
                err(&tr(&format!("메모 저장: {name}"), &format!("Note saved: {name}"), &format!("已保存备注：{name}")));
            }
            Ok(0)
        }

        "get" => {
            if rest.len() != 1 {
                return Ok(bad());
            }
            if !authorize_read(store, &rest)? {
                return Ok(denied());
            }
            println!("{}", store.get(rest[0])?);
            Ok(0)
        }

        "exec" => {
            let Some(sep) = rest.iter().position(|a| *a == "--") else { return Ok(bad()) };
            let names = &rest[..sep];
            let command = &rest[sep + 1..];
            if names.is_empty() || command.is_empty() {
                return Ok(bad());
            }
            if !authorize_read(store, names)? {
                return Ok(denied());
            }
            let mut env = Vec::with_capacity(names.len());
            for name in names {
                env.push((name.to_string(), store.get(name)?));
            }
            Ok(exec(command, env))
        }

        "set" => {
            if !(rest.len() == 1 || (rest.len() == 3 && rest[1] == "--note")) {
                return Ok(bad());
            }
            let name = rest[0];
            let note = (rest.len() == 3).then(|| rest[2]);
            if !is_valid_name(name) {
                return Err(Error::InvalidName(name.to_string()));
            }
            if store.exists(name)?
                && !store.authenticate(&tr(&format!("'{name}' 시크릿을 덮어쓰려고 합니다"), &format!("overwrite '{name}'"), &format!("覆盖 '{name}'")))
            {
                return Ok(denied());
            }
            let value = match read_secret() {
                Some(v) if !v.is_empty() => v,
                _ => {
                    err(&tr("값이 비어 있습니다", "The value is empty", "值为空"));
                    return Ok(1);
                }
            };
            store.set(name, &value)?;
            if let Some(note) = note {
                store.set_note(name, Some(note))?;
            }
            err(&tr(&format!("저장됨: {name}"), &format!("Saved: {name}"), &format!("已保存：{name}")));
            Ok(0)
        }

        "rm" => {
            if rest.len() != 1 {
                return Ok(bad());
            }
            let name = rest[0];
            if !store.exists(name)? {
                return Err(Error::NotFound(name.to_string()));
            }
            if !store.authenticate(&tr(&format!("'{name}' 시크릿을 삭제하려고 합니다"), &format!("delete '{name}'"), &format!("删除 '{name}'"))) {
                return Ok(denied());
            }
            store.remove(name)?;
            err(&tr(&format!("삭제됨: {name}"), &format!("Deleted: {name}"), &format!("已删除：{name}")));
            Ok(0)
        }

        "mv" | "rename" => {
            if rest.len() != 2 {
                return Ok(bad());
            }
            let (old, new) = (rest[0], rest[1]);
            if !is_valid_name(new) {
                return Err(Error::InvalidName(new.to_string()));
            }
            if !store.exists(old)? {
                return Err(Error::NotFound(old.to_string()));
            }
            if store.exists(new)? {
                return Err(Error::AlreadyExists(new.to_string()));
            }
            if !store.authenticate(&tr(
                &format!("'{old}' 시크릿의 이름을 '{new}'(으)로 바꾸려고 합니다"),
                &format!("rename '{old}' to '{new}'"),
                &format!("将 '{old}' 重命名为 '{new}'"),
            )) {
                return Ok(denied());
            }
            store.rename(old, new)?;
            err(&tr(&format!("이름 변경: {old} → {new}"), &format!("Renamed: {old} → {new}"), &format!("已重命名：{old} → {new}")));
            Ok(0)
        }

        "allow" => {
            if !(rest.len() == 1 || (rest.len() == 3 && rest[1] == "--hours")) {
                return Ok(bad());
            }
            let name = rest[0];
            let all = name == "--all";
            if !all && !store.exists(name)? {
                return Err(Error::NotFound(name.to_string()));
            }
            let mut until = 0.0;
            let mut label = tr("무기한", "no expiry", "不过期");
            if rest.len() == 3 {
                match rest[2].parse::<f64>() {
                    Ok(hours) if hours > 0.0 && hours <= 24.0 * 365.0 => {
                        until = now() + hours * 3600.0;
                        label = tr(&format!("{}시간", rest[2]), &format!("{} hours", rest[2]), &format!("{} 小时", rest[2]));
                    }
                    _ => return Ok(bad()),
                }
            }
            if all {
                if !store.authenticate(&tr(
                    &format!("모든 시크릿(앞으로 추가될 것 포함)을 인증 없이 읽도록 허용({label})하려고 합니다"),
                    &format!("let agents read every secret, including future ones, without asking ({label})"),
                    &format!("允许助手无需验证读取所有密钥（包括以后添加的）（{label}）"),
                )) {
                    return Ok(denied());
                }
                store.set_all_grant(Some(until))?;
                err(&tr(&format!("전체 허용됨 ({label})"), &format!("All allowed ({label})"), &format!("已全部允许（{label}）")));
            } else {
                if !store.authenticate(&tr(
                    &format!("'{name}' 시크릿을 인증 없이 읽도록 허용({label})하려고 합니다"),
                    &format!("let agents read '{name}' without asking ({label})"),
                    &format!("允许助手无需验证读取 '{name}'（{label}）"),
                )) {
                    return Ok(denied());
                }
                store.set_grant(name, Some(until))?;
                err(&tr(&format!("허용됨: {name} ({label})"), &format!("Allowed: {name} ({label})"), &format!("已允许：{name}（{label}）")));
            }
            Ok(0)
        }

        "deny" => {
            if rest.len() != 1 {
                return Ok(bad());
            }
            if rest[0] == "--all" {
                store.set_all_grant(None)?;
                err(&tr("전체 허용 해제: 모든 시크릿이 다시 매번 묻기", "All access revoked: every secret asks again", "已撤销全部访问：所有密钥恢复每次询问"));
            } else {
                store.set_grant(rest[0], None)?;
                err(&tr(&format!("허용 해제: {}", rest[0]), &format!("Revoked: {}", rest[0]), &format!("已撤销：{}", rest[0])));
            }
            Ok(0)
        }

        "help" | "-h" | "--help" => {
            println!("{}", usage());
            Ok(0)
        }

        _ => Ok(bad()),
    }
}

/// 허용되지 않은 이름이 하나라도 있으면 한 번의 인증으로 묶어서 묻는다.
fn authorize_read(store: &dyn SecretStore, names: &[&str]) -> Result<bool, Error> {
    let known = store.names()?;
    for name in names {
        if !is_valid_name(name) {
            return Err(Error::InvalidName(name.to_string()));
        }
        if !known.iter().any(|k| k == name) {
            return Err(Error::NotFound(name.to_string()));
        }
    }
    let grants = store.grants();
    let t = now();
    let pending: Vec<&str> = names.iter().copied().filter(|n| !grants::is_granted(n, &grants, t)).collect();
    if pending.is_empty() {
        return Ok(true);
    }
    let list = pending.join(", ");
    Ok(store.authenticate(&tr(&format!("시크릿을 읽으려고 합니다: {list}"), &format!("read secrets: {list}"), &format!("读取密钥：{list}"))))
}

fn read_secret() -> Option<String> {
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        // 파이프 입력: 전체를 읽고 끝의 개행만 제거
        let mut text = String::new();
        stdin.lock().read_to_string(&mut text).ok()?;
        while text.ends_with('\n') || text.ends_with('\r') {
            text.pop();
        }
        return Some(text);
    }
    let _ = write!(std::io::stderr(), "{}", tr("값(입력은 표시되지 않음): ", "Value (input hidden): ", "值（输入不会显示）："));
    let _ = std::io::stderr().flush();
    rpassword::read_password().ok()
}

/// 시크릿을 환경변수로 넣어 명령을 하위 프로세스로 실행한다.
/// - 명령의 표준 출력·표준 오류에서 시크릿 값을 `***` 로 가린다 (에이전트 대화 기록에 값이 남지 않게).
///   명령 자체는 값을 그대로 쓴다. 표준 입력은 그대로 넘긴다.
/// - 명령의 종료 코드를 그대로 돌려준다 (신호로 끝나면 128 + 신호 번호).
/// - Ctrl+C 는 같은 터미널의 하위 프로세스가 받고, akey 는 무시하고 기다린다.
fn exec(command: &[&str], env: Vec<(String, String)>) -> i32 {
    use std::process::{Command, Stdio};

    let secrets: Vec<String> = env.iter().map(|(_, v)| v.clone()).collect();
    let mut cmd = Command::new(command[0]);
    cmd.args(&command[1..]).envs(env).stdin(Stdio::inherit()).stdout(Stdio::piped()).stderr(Stdio::piped());

    ignore_interrupts();
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            err(&format!("{}{error}", tr(&format!("실행 실패: {}: ", command[0]), &format!("Failed to run {}: ", command[0]), &format!("无法运行 {}：", command[0]))));
            return 127;
        }
    };

    let out = child.stdout.take().map(|pipe| {
        let secrets = secrets.clone();
        std::thread::spawn(move || pump(pipe, std::io::stdout(), &secrets))
    });
    let errs = child.stderr.take().map(|pipe| {
        let secrets = secrets.clone();
        std::thread::spawn(move || pump(pipe, std::io::stderr(), &secrets))
    });
    let status = child.wait();
    for t in [out, errs].into_iter().flatten() {
        let _ = t.join();
    }

    match status {
        Ok(status) => {
            if let Some(code) = status.code() {
                return code;
            }
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                if let Some(signal) = status.signal() {
                    return 128 + signal;
                }
            }
            1
        }
        Err(_) => 1,
    }
}

/// 하위 프로세스 출력을 읽어 시크릿을 가린 뒤 내보낸다
fn pump(mut from: impl Read, mut to: impl Write, secrets: &[String]) {
    let mut masker = mask::Masker::new(secrets.iter().map(|s| s.as_bytes()));
    let mut buf = [0u8; 8192];
    loop {
        match from.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let ready = masker.push(&buf[..n]);
                if to.write_all(&ready).and_then(|_| to.flush()).is_err() {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    let _ = to.write_all(&masker.finish());
    let _ = to.flush();
}

/// akey 는 Ctrl+C·종료 신호를 받아도 끝나지 않고 하위 프로세스를 기다린다 (하위 프로세스는 신호를 받는다).
/// Unix 는 '무시'가 아닌 빈 처리기를 건다: 무시(SIG_IGN)는 하위 프로세스에 그대로 물려지기 때문.
fn ignore_interrupts() {
    #[cfg(unix)]
    unsafe {
        extern "C" fn noop(_: libc::c_int) {}
        for sig in [libc::SIGINT, libc::SIGQUIT, libc::SIGTERM, libc::SIGHUP] {
            libc::signal(sig, noop as extern "C" fn(libc::c_int) as libc::sighandler_t);
        }
    }
    #[cfg(windows)]
    akey_windows::ignore_ctrl_c();
}
