//! 여러 시크릿을 공유용 `.env` 형식(`NAME=value` 줄)으로 만든다.

/// 값에 공백·따옴표·`#`·`$`·백틱·`\`·줄바꿈 등이 있으면 큰따옴표로 감싸고 이스케이프한다.
/// (`$` 와 백틱은 셸에서 `source .env` 할 때 명령 실행으로 이어질 수 있어 반드시 막는다)
/// 받는 쪽에서 `.env` 파일이나 dotenv 라이브러리로 그대로 읽을 수 있게 한다.
pub fn quote(value: &str) -> String {
    let plain = !value.is_empty()
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:@+,=%~".contains(c));
    if plain {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '$' => out.push_str("\\$"),
            '`' => out.push_str("\\`"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// (이름, 값) 목록 → 줄마다 `NAME=value`, 끝에 줄바꿈
pub fn format(pairs: &[(String, String)]) -> String {
    let mut out = String::new();
    for (name, value) in pairs {
        out.push_str(name);
        out.push('=');
        out.push_str(&quote(value));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_values_stay_unquoted() {
        assert_eq!(quote("sk-proj-AbC123_x.y/z"), "sk-proj-AbC123_x.y/z");
        assert_eq!(quote("postgres://u:p@host:5432/db"), "postgres://u:p@host:5432/db");
    }

    #[test]
    fn special_values_are_quoted_and_escaped() {
        assert_eq!(quote(""), "\"\"");
        assert_eq!(quote("a b"), "\"a b\"");
        assert_eq!(quote("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quote("line1\nline2"), "\"line1\\nline2\"");
        assert_eq!(quote("c:\\path"), "\"c:\\\\path\"");
        assert_eq!(quote("$HOME#x"), "\"\\$HOME#x\"");
        assert_eq!(quote("a`id`b"), "\"a\\`id\\`b\"");
    }

    #[test]
    fn formats_lines_in_given_order() {
        let pairs = vec![("B_KEY".to_string(), "2".to_string()), ("A_KEY".to_string(), "x y".to_string())];
        assert_eq!(format(&pairs), "B_KEY=2\nA_KEY=\"x y\"\n");
    }
}
