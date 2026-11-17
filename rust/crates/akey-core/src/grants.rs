//! 에이전트 허용 계산과 표시 문구. 값: 만료 시각(epoch 초), 0 = 무기한.

use std::collections::BTreeMap;

use crate::i18n::tr;
use crate::vault::ALL_KEY;

pub type Grants = BTreeMap<String, f64>;

pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn active(until: f64, now: f64) -> bool {
    until == 0.0 || until > now
}

/// 지금 유효한 허용의 만료 시각. 개별 허용과 전체 허용 중 더 긴 쪽. None = 허용 없음, Some(0) = 무기한
pub fn effective_until(name: &str, grants: &Grants, now: f64) -> Option<f64> {
    let candidates: Vec<f64> = [grants.get(name), grants.get(ALL_KEY)]
        .into_iter()
        .flatten()
        .copied()
        .filter(|u| active(*u, now))
        .collect();
    if candidates.contains(&0.0) {
        return Some(0.0);
    }
    candidates.into_iter().reduce(f64::max)
}

pub fn is_granted(name: &str, grants: &Grants, now: f64) -> bool {
    effective_until(name, grants, now).is_some()
}

/// 남은 시간은 올림: 8시간 허용 직후에 "7시간 남음"으로 보이지 않게
pub fn label(until: Option<f64>, now: f64) -> String {
    let ask = tr("매번 묻기", "Ask every time", "每次询问");
    let Some(until) = until else { return ask };
    if until == 0.0 {
        return tr("항상 허용", "Always allowed", "始终允许");
    }
    let remaining = until - now;
    if remaining <= 0.0 {
        return ask;
    }
    if remaining >= 3600.0 {
        let hours = (remaining / 3600.0).ceil() as u64;
        return tr(&format!("{hours}시간 남음"), &format!("{hours}h left"), &format!("剩余 {hours} 小时"));
    }
    let minutes = (remaining / 60.0).ceil() as u64;
    tr(&format!("{minutes}분 남음"), &format!("{minutes}m left"), &format!("剩余 {minutes} 分钟"))
}

pub fn grant_label(name: &str, grants: &Grants, now: f64) -> String {
    let effective = effective_until(name, grants, now);
    let text = label(effective, now);
    // 개별 허용은 없고 전체 허용 때문에 열려 있으면 표시
    let own = grants.get(name).copied().filter(|u| active(*u, now));
    if own.is_none() && effective.is_some() {
        return text + &tr(" (전체)", " (all)", "（全部）");
    }
    text
}

/// 전체 허용 상태 라벨. 없으면 None
pub fn all_grant_label(grants: &Grants, now: f64) -> Option<String> {
    let until = *grants.get(ALL_KEY)?;
    active(until, now).then(|| label(Some(until), now))
}

/// 만료된 허용만 골라 낸다 (Windows 에서 다시 봉인할 대상)
pub fn expired(grants: &Grants, now: f64) -> Vec<String> {
    grants.iter().filter(|(_, u)| !active(**u, now)).map(|(k, _)| k.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{set_lang, Lang};

    /// 언어는 전역 상태라 언어를 바꾸는 테스트는 하나씩 돌린다
    static LANG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn lang_guard(lang: Lang) -> std::sync::MutexGuard<'static, ()> {
        let guard = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_lang(lang);
        guard
    }

    fn g(pairs: &[(&str, f64)]) -> Grants {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn labels_round_up() {
        let _g = lang_guard(Lang::Ko);
        let now = 1_000_000.0;
        assert_eq!(label(Some(now + 8.0 * 3600.0 - 2.0), now), "8시간 남음");
        assert_eq!(label(Some(now + 3599.0), now), "60분 남음");
        assert_eq!(label(Some(now + 30.0), now), "1분 남음");
        assert_eq!(label(Some(now - 1.0), now), "매번 묻기");
        assert_eq!(label(Some(0.0), now), "항상 허용");
        assert_eq!(label(None, now), "매번 묻기");
    }

    #[test]
    fn all_grant_combines_with_individual() {
        let _g = lang_guard(Lang::Ko);
        let now = 1_000_000.0;
        let grants = g(&[("A", now + 2.0 * 3600.0), (ALL_KEY, now + 8.0 * 3600.0 - 1.0)]);
        assert_eq!(effective_until("A", &grants, now), Some(now + 8.0 * 3600.0 - 1.0));
        assert_eq!(grant_label("A", &grants, now), "8시간 남음");
        assert_eq!(grant_label("B", &grants, now), "8시간 남음 (전체)");
        assert_eq!(all_grant_label(&grants, now).as_deref(), Some("8시간 남음"));

        let forever = g(&[(ALL_KEY, 0.0)]);
        assert_eq!(effective_until("X", &forever, now), Some(0.0));
        assert_eq!(grant_label("X", &forever, now), "항상 허용 (전체)");

        let expired_all = g(&[(ALL_KEY, now - 10.0)]);
        assert!(!is_granted("X", &expired_all, now));
        assert_eq!(all_grant_label(&expired_all, now), None);
        assert_eq!(expired(&expired_all, now), vec!["*".to_string()]);
    }

    #[test]
    fn chinese_labels() {
        let _g = lang_guard(Lang::Zh);
        let now = 0.0;
        assert_eq!(label(Some(3600.0 * 3.0), now), "剩余 3 小时");
        assert_eq!(label(None, now), "每次询问");
    }

    #[test]
    fn english_labels() {
        let _g = lang_guard(Lang::En);
        let now = 0.0;
        assert_eq!(label(Some(3600.0 * 3.0), now), "3h left");
        assert_eq!(label(Some(0.0), now), "Always allowed");
    }
}
