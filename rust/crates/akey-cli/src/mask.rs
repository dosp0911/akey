//! `akey exec` 로 실행한 명령의 출력에서 시크릿 값을 `***` 로 가린다.
//!
//! 출력은 조각(chunk) 단위로 들어오므로 값이 두 조각에 걸쳐 나올 수 있다.
//! 그래서 버퍼 끝부분이 어떤 시크릿의 앞부분과 같을 때만 그만큼 붙잡아 두고, 나머지는 바로 내보낸다
//! (일반 출력은 늦어지지 않는다).

pub const MASK: &[u8] = b"***";
/// 이보다 짧은 값은 가리지 않는다 (흔한 글자까지 망가뜨리지 않게)
pub const MIN_SECRET_LEN: usize = 4;

pub struct Masker {
    /// 긴 것부터 (긴 값이 짧은 값을 포함할 때 긴 값 전체를 가리도록)
    secrets: Vec<Vec<u8>>,
    pending: Vec<u8>,
}

impl Masker {
    pub fn new<I, S>(secrets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<[u8]>,
    {
        let mut secrets: Vec<Vec<u8>> = secrets
            .into_iter()
            .map(|s| s.as_ref().to_vec())
            .filter(|s| s.len() >= MIN_SECRET_LEN)
            .collect();
        secrets.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        secrets.dedup();
        Masker { secrets, pending: Vec::new() }
    }

    /// 새 조각을 넣고, 지금 내보내도 되는 바이트를 돌려준다
    pub fn push(&mut self, chunk: &[u8]) -> Vec<u8> {
        if self.secrets.is_empty() {
            return chunk.to_vec();
        }
        self.pending.extend_from_slice(chunk);
        let replaced = self.replace_all(&self.pending);
        let hold = self.held_suffix_len(&replaced);
        let cut = replaced.len() - hold;
        self.pending = replaced[cut..].to_vec();
        replaced[..cut].to_vec()
    }

    /// 출력이 끝났을 때 남은 것을 내보낸다
    pub fn finish(&mut self) -> Vec<u8> {
        let rest = std::mem::take(&mut self.pending);
        self.replace_all(&rest)
    }

    fn replace_all(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        let mut i = 0;
        'outer: while i < data.len() {
            for s in &self.secrets {
                if data[i..].starts_with(s) {
                    out.extend_from_slice(MASK);
                    i += s.len();
                    continue 'outer;
                }
            }
            out.push(data[i]);
            i += 1;
        }
        out
    }

    /// 버퍼 끝부분 중 어떤 시크릿의 (전체가 아닌) 앞부분과 같은 가장 긴 길이
    fn held_suffix_len(&self, data: &[u8]) -> usize {
        let mut best = 0;
        for s in &self.secrets {
            let max = (s.len() - 1).min(data.len());
            for k in (best + 1..=max).rev() {
                if data.ends_with(&s[..k]) {
                    best = k;
                    break;
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(secrets: &[&str], chunks: &[&str]) -> String {
        let mut m = Masker::new(secrets.iter().map(|s| s.as_bytes()));
        let mut out = Vec::new();
        for c in chunks {
            out.extend(m.push(c.as_bytes()));
        }
        out.extend(m.finish());
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn masks_whole_value_in_one_chunk() {
        assert_eq!(run(&["sk-secret-123"], &["key=sk-secret-123 ok\n"]), "key=*** ok\n");
    }

    #[test]
    fn masks_value_split_across_chunks() {
        assert_eq!(run(&["sk-secret-123"], &["key=sk-sec", "ret-1", "23 ok\n"]), "key=*** ok\n");
        assert_eq!(run(&["abcd"], &["a", "b", "c", "d", "\n"]), "***\n");
    }

    #[test]
    fn masks_every_occurrence_and_several_secrets() {
        assert_eq!(run(&["tok1", "pass-xyz"], &["tok1 tok1 pass-xyz\n"]), "*** *** ***\n");
    }

    #[test]
    fn longer_secret_wins_when_one_contains_another() {
        assert_eq!(run(&["abcd", "abcdefgh"], &["[abcdefgh]"]), "[***]");
    }

    #[test]
    fn plain_output_is_not_held_back() {
        let mut m = Masker::new(["sk-secret-123"]);
        assert_eq!(m.push(b"hello world\n"), b"hello world\n".to_vec());
        // 끝이 시크릿 앞부분일 때만 그만큼 붙잡는다
        assert_eq!(m.push(b"partial sk-"), b"partial ".to_vec());
        assert_eq!(m.push(b"nope\n"), b"sk-nope\n".to_vec());
    }

    #[test]
    fn short_values_are_not_masked() {
        assert_eq!(run(&["abc"], &["abc abc\n"]), "abc abc\n");
    }

    #[test]
    fn no_secrets_passes_through() {
        assert_eq!(run(&[], &["x", "y"]), "xy");
    }
}
