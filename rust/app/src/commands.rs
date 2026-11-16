//! 화면(JS)이 부르는 명령. Swift 판 Model 과 같은 규칙·문구.
//! 반환값 Ok(String) = 상태줄에 보일 메시지, Err(String) = 오류 메시지 (둘 다 현재 언어).

use akey_core::grants::{self, now};
use akey_core::{is_valid_name, tr, Error, ALL_KEY};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::session::{AppState, EVENT_CHANGED};

type CmdResult<T> = Result<T, String>;

fn e(err: Error) -> String {
    err.to_string()
}

fn cancelled() -> String {
    tr("취소됨", "Cancelled")
}

fn changed(app: &AppHandle) {
    let _ = app.emit(EVENT_CHANGED, ());
}

#[derive(Serialize)]
pub struct Item {
    name: String,
    note: Option<String>,
    grant_label: String,
    /// ask | timed | always
    grant_kind: &'static str,
}

#[derive(Serialize)]
pub struct Snapshot {
    lang: &'static str,
    items: Vec<Item>,
    all_label: Option<String>,
    all_kind: &'static str,
    unlocked: bool,
    minutes_left: Option<u64>,
    unlock_minutes: u32,
    error: Option<String>,
}

fn kind(until: Option<f64>) -> &'static str {
    match until {
        None => "ask",
        Some(u) if u == 0.0 => "always",
        Some(_) => "timed",
    }
}

#[tauri::command]
pub fn snapshot(state: State<'_, AppState>) -> Snapshot {
    let t = now();
    let grants = state.store.grants();
    let mut error = None;
    let names = state.store.names().unwrap_or_else(|err| {
        error = Some(err.to_string());
        Vec::new()
    });
    let notes = state.store.notes().unwrap_or_default();
    let items = names
        .into_iter()
        .map(|name| {
            let until = grants::effective_until(&name, &grants, t);
            Item {
                note: notes.get(&name).cloned(),
                grant_label: grants::grant_label(&name, &grants, t),
                grant_kind: kind(until),
                name,
            }
        })
        .collect();
    let all_until = grants.get(ALL_KEY).copied().filter(|u| *u == 0.0 || *u > t);
    let settings = state.settings();
    Snapshot {
        lang: akey_core::i18n::lang().code(),
        items,
        all_label: grants::all_grant_label(&grants, t),
        all_kind: kind(all_until),
        unlocked: state.is_unlocked(),
        minutes_left: state.minutes_left(),
        unlock_minutes: settings.unlock_minutes,
        error,
    }
}

fn keep_note(state: &AppState) -> String {
    let m = state.settings().unlock_minutes;
    tr(&format!("{m}분간 다시 묻지 않음"), &format!("won't ask again for {m} min"))
}

#[tauri::command]
pub async fn reveal(app: AppHandle, state: State<'_, AppState>, name: String) -> CmdResult<String> {
    let reason = tr(&format!("시크릿 값을 보려고 합니다 ({})", keep_note(&state)), &format!("view a secret value ({})", keep_note(&state)));
    if !state.unlock(&app, reason).await {
        return Err(cancelled());
    }
    state.store.get(&name).map_err(e)
}

#[tauri::command]
pub async fn copy(app: AppHandle, state: State<'_, AppState>, name: String) -> CmdResult<String> {
    let reason = tr(
        &format!("'{name}' 시크릿을 복사하려고 합니다 ({})", keep_note(&state)),
        &format!("copy '{name}' ({})", keep_note(&state)),
    );
    if !state.unlock(&app, reason).await {
        return Err(cancelled());
    }
    let value = state.store.get(&name).map_err(e)?;
    crate::platform::copy_to_clipboard(&value, state.settings().conceal_clipboard)?;
    Ok(tr(&format!("복사됨: {name}"), &format!("Copied: {name}")))
}

/// 여러 시크릿을 `.env` 형식(NAME=value 줄)으로 한 번에 복사한다 (다른 사람에게 공유용).
/// 잠금 해제 상태와 상관없이 **매번** 인증한다.
#[tauri::command]
pub async fn copy_many(app: AppHandle, state: State<'_, AppState>, names: Vec<String>) -> CmdResult<String> {
    if names.is_empty() {
        return Err(tr("복사할 시크릿을 고르세요", "Select secrets to copy"));
    }
    let n = names.len();
    let list = names.join(", ");
    let reason = tr(
        &format!("시크릿 {n}개를 공유용으로 복사하려고 합니다: {list}"),
        &format!("copy {n} secrets for sharing: {list}"),
    );
    if !state.confirm(&app, reason).await {
        return Err(cancelled());
    }
    let result = names
        .iter()
        .map(|name| state.store.get(name).map(|v| (name.clone(), v)))
        .collect::<Result<Vec<_>, _>>();
    state.relock_if_no_session();
    let pairs = result.map_err(e)?;
    crate::platform::copy_to_clipboard(&akey_core::dotenv::format(&pairs), state.settings().conceal_clipboard)?;
    Ok(tr(&format!("{n}개 복사됨 (.env 형식)"), &format!("Copied {n} secrets (.env format)")))
}

#[tauri::command]
pub fn lock(app: AppHandle, state: State<'_, AppState>) {
    state.lock();
    changed(&app);
}

#[tauri::command]
pub async fn add(app: AppHandle, state: State<'_, AppState>, name: String, value: String, note: String) -> CmdResult<String> {
    let name = name.trim().to_string();
    if !is_valid_name(&name) {
        return Err(tr("이름은 영문/숫자/_ 만 가능합니다", "Names may use letters, digits and _ only"));
    }
    if value.is_empty() {
        return Err(tr("값이 비어 있습니다", "The value is empty"));
    }
    let exists = state.store.exists(&name).map_err(e)?;
    if exists {
        if !state.confirm(&app, tr(&format!("'{name}' 시크릿을 덮어쓰려고 합니다"), &format!("overwrite '{name}'"))).await {
            return Err(cancelled());
        }
    }
    let result = (|| {
        state.store.set(&name, &value)?;
        if !note.trim().is_empty() {
            state.store.set_note(&name, Some(&note))?;
        }
        Ok::<_, Error>(())
    })();
    // 처음 저장할 때 Windows Hello 키를 만들 수 있다. 세션이 없으면 기억한 키를 지운다
    state.relock_if_no_session();
    result.map_err(e)?;
    changed(&app);
    Ok(tr(&format!("저장됨: {name}"), &format!("Saved: {name}")))
}

/// 수정 저장. 이름이나 값이 바뀌면 매번 인증, 메모만 바뀌면 인증 없음. value 가 비면 값 유지.
#[tauri::command]
pub async fn save_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    original: String,
    name: String,
    value: String,
    note: String,
) -> CmdResult<String> {
    let new_name = name.trim().to_string();
    if !is_valid_name(&new_name) {
        return Err(tr(
            "이름은 영문/숫자/_ 만 가능합니다 (숫자로 시작 불가)",
            "Names may use letters, digits and _ only (not starting with a digit)",
        ));
    }
    if new_name != original && state.store.exists(&new_name).map_err(e)? {
        return Err(tr(&format!("'{new_name}' 이(가) 이미 있습니다"), &format!("'{new_name}' already exists")));
    }
    let renaming = new_name != original;
    let changing_value = !value.is_empty();
    if renaming || changing_value {
        let reason = match (renaming, changing_value) {
            (true, true) => tr(
                &format!("'{original}' 시크릿의 이름을 '{new_name}'(으)로, 값을 바꾸려고 합니다"),
                &format!("rename '{original}' to '{new_name}' and change its value"),
            ),
            (true, false) => tr(
                &format!("'{original}' 시크릿의 이름을 '{new_name}'(으)로 바꾸려고 합니다"),
                &format!("rename '{original}' to '{new_name}'"),
            ),
            _ => tr(&format!("'{original}' 시크릿의 값을 바꾸려고 합니다"), &format!("change the value of '{original}'")),
        };
        if !state.confirm(&app, reason).await {
            return Err(cancelled());
        }
    }
    let result = state.store.update(&original, &new_name, changing_value.then_some(value.as_str()), Some(&note));
    if renaming || changing_value {
        state.relock_if_no_session();
    }
    result.map_err(e)?;
    changed(&app);
    Ok(tr(&format!("수정됨: {new_name}"), &format!("Updated: {new_name}")))
}

#[tauri::command]
pub async fn remove(app: AppHandle, state: State<'_, AppState>, name: String) -> CmdResult<String> {
    if !state.confirm(&app, tr(&format!("'{name}' 시크릿을 삭제하려고 합니다"), &format!("delete '{name}'"))).await {
        return Err(cancelled());
    }
    let result = state.store.remove(&name);
    state.relock_if_no_session();
    result.map_err(e)?;
    changed(&app);
    Ok(tr(&format!("삭제됨: {name}"), &format!("Deleted: {name}")))
}

fn duration_label(hours: f64) -> String {
    if hours == 0.0 {
        tr("무기한", "no expiry")
    } else {
        tr(&format!("{}시간", hours as u64), &format!("{} hours", hours as u64))
    }
}

/// hours: None = 매번 묻기, 0 = 항상 허용, 그 외 = 해당 시간 동안 허용
#[tauri::command]
pub async fn set_grant(app: AppHandle, state: State<'_, AppState>, name: String, hours: Option<f64>) -> CmdResult<String> {
    match hours {
        Some(hours) => {
            let label = duration_label(hours);
            let reason = tr(
                &format!("'{name}' 시크릿을 인증 없이 읽도록 허용({label})하려고 합니다"),
                &format!("let agents read '{name}' without asking ({label})"),
            );
            if !state.confirm(&app, reason).await {
                return Err(cancelled());
            }
            let until = if hours == 0.0 { 0.0 } else { now() + hours * 3600.0 };
            let result = state.store.set_grant(&name, Some(until));
            state.relock_if_no_session();
            result.map_err(e)?;
        }
        None => state.store.set_grant(&name, None).map_err(e)?,
    }
    changed(&app);
    Ok(String::new())
}

/// 전체 허용. hours: None = 전체 허용만 끔 (시크릿별 허용은 그대로), 0 = 무기한
#[tauri::command]
pub async fn set_all_grant(app: AppHandle, state: State<'_, AppState>, hours: Option<f64>) -> CmdResult<String> {
    let message = match hours {
        Some(hours) => {
            let label = duration_label(hours);
            let reason = tr(
                &format!("모든 시크릿(앞으로 추가될 것 포함)을 인증 없이 읽도록 허용({label})하려고 합니다"),
                &format!("let agents read every secret, including future ones, without asking ({label})"),
            );
            if !state.confirm(&app, reason).await {
                return Err(cancelled());
            }
            let until = if hours == 0.0 { 0.0 } else { now() + hours * 3600.0 };
            let result = state.store.set_all_grant(Some(until));
            state.relock_if_no_session();
            result.map_err(e)?;
            tr(&format!("전체 허용됨 ({label})"), &format!("All allowed ({label})"))
        }
        None => {
            state.store.unset_all_grant().map_err(e)?;
            tr("전체 허용 꺼짐 (시크릿별 설정은 유지)", "All-access off (per-secret access kept)")
        }
    };
    changed(&app);
    Ok(message)
}

/// 모든 허용 해제: 전체 허용과 시크릿별 허용을 모두 지워 전부 '매번 묻기'로 (인증 필요 없음, 권한이 줄기만 함)
#[tauri::command]
pub fn revoke_all_grants(app: AppHandle, state: State<'_, AppState>) -> CmdResult<String> {
    state.store.set_all_grant(None).map_err(e)?;
    changed(&app);
    Ok(tr("모든 허용 해제: 전부 매번 묻기", "All access revoked: every secret asks again"))
}

// MARK: 설정

#[derive(Serialize)]
pub struct SettingsView {
    lang: &'static str,
    language: &'static str,
    unlock_minutes: u32,
    unlock_choices: Vec<u32>,
    conceal_clipboard: bool,
    autostart: bool,
    all_label: Option<String>,
    all_kind: &'static str,
    /// 지금 유효한 시크릿별 허용 개수 (전체 허용 제외)
    individual_count: usize,
    windows_hello: bool,
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> SettingsView {
    use tauri_plugin_autostart::ManagerExt;
    let s = state.settings();
    let t = now();
    let grants = state.store.grants();
    let all_until = grants.get(ALL_KEY).copied().filter(|u| *u == 0.0 || *u > t);
    SettingsView {
        lang: akey_core::i18n::lang().code(),
        language: s.effective_language().code(),
        unlock_minutes: s.unlock_minutes,
        unlock_choices: akey_core::settings::UNLOCK_CHOICES.to_vec(),
        conceal_clipboard: s.conceal_clipboard,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        all_label: grants::all_grant_label(&grants, t),
        all_kind: kind(all_until),
        individual_count: grants.iter().filter(|(k, u)| k.as_str() != ALL_KEY && (**u == 0.0 || **u > t)).count(),
        windows_hello: cfg!(windows),
    }
}

#[tauri::command]
pub fn set_language(app: AppHandle, state: State<'_, AppState>, code: String) -> CmdResult<()> {
    let lang = akey_core::Lang::from_code(&code).ok_or("invalid language")?;
    state.update_settings(|s| s.language = Some(lang)).map_err(|e| e.to_string())?;
    akey_core::i18n::set_lang(lang);
    crate::tray::refresh_menu(&app);
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_title(&tr("akey 설정", "akey Settings"));
    }
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_unlock_minutes(app: AppHandle, state: State<'_, AppState>, minutes: u32) -> CmdResult<()> {
    if !akey_core::settings::UNLOCK_CHOICES.contains(&minutes) {
        return Err("invalid".into());
    }
    state.update_settings(|s| s.unlock_minutes = minutes).map_err(|e| e.to_string())?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_conceal_clipboard(app: AppHandle, state: State<'_, AppState>, on: bool) -> CmdResult<()> {
    state.update_settings(|s| s.conceal_clipboard = on).map_err(|e| e.to_string())?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, on: bool) -> CmdResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let result = if on { launcher.enable() } else { launcher.disable() };
    changed(&app);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    crate::app::open_settings(&app);
}

/// 팝오버 크기를 내용에 맞춘 뒤 트레이 기준 위치를 다시 잡는다
#[tauri::command]
pub fn fit_popover(app: AppHandle, height: f64) {
    let Some(w) = app.get_webview_window("main") else { return };
    let height = height.clamp(160.0, 900.0);
    // macOS: 크기만 바꾸고 위쪽은 제자리 (아이콘 아래 위치는 띄울 때 정한다)
    #[cfg(target_os = "macos")]
    {
        let win = w.clone();
        let _ = w.run_on_main_thread(move || {
            if let Ok(ptr) = win.ns_window() {
                unsafe { akey_macos::status::resize_popover_keep_top(ptr, 540.0, height) };
            }
        });
        crate::app::wake();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = w.set_size(tauri::LogicalSize::new(540.0, height));
        crate::app::position_popover(&w);
    }
}

/// 화면(JS) 쪽 문제 확인용 기록 (AKEY_DEBUG_LOG 가 있을 때만 남는다)
#[tauri::command]
pub fn debug_log(message: String) {
    crate::app::debug(&format!("ui: {}", message.chars().take(300).collect::<String>()));
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    crate::app::hide_popover(&app);
}
