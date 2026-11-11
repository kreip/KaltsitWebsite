//! Writing one XUnity dictionary line so that the game's decoder
//! ([`crate::translators::import::xunity::decode_line`]) reads back the very
//! key and value (the rules: translators/README.md, "XUnity dictionary and
//! plugin format"):
//!
//! * `\` -> `\\`, `=` -> `\=`, LF -> `\n`, CR -> `\r`;
//! * the second `/` of a `//` -> `\u002F` (else a comment would start);
//! * `%` before `3D` -> `\u0025` (else `%3D` would read as `=`);
//! * the first `#` of a line that would read as a `#set` / `#unset` /
//!   `#enable` directive -> `\u0023`;
//! * a key starting with `r:` / `sr:` (a regex for XUnity) or an empty key or
//!   value cannot be written: it goes to the export report.

/// Why a translation cannot be written to a dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unwritable {
    EmptyKey,
    EmptyValue,
    /// XUnity reads a key starting with `r:` / `sr:` as a regex.
    RegexKey,
}

impl Unwritable {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyKey => "empty key",
            Self::EmptyValue => "empty value",
            Self::RegexKey => "key starts with r: or sr: (a regex for XUnity)",
        }
    }
}

fn encode_part(s: &str, out: &mut String) {
    let chars: Vec<char> = s.chars().collect();
    let mut last_slash = false;
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\\' => out.push_str("\\\\"),
            '=' => out.push_str("\\="),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '/' if last_slash => {
                out.push_str("\\u002F");
                last_slash = false;
                continue;
            }
            '%' if matches!(chars.get(i + 1..i + 3), Some(['3', 'D' | 'd'])) => {
                out.push_str("\\u0025");
            }
            _ => out.push(c),
        }
        last_slash = c == '/';
    }
}

/// A value alone, for a line whose key part is kept exactly as in the
/// pack's file (`<key as written>=` + this).
pub fn encode_value(value: &str) -> Result<String, Unwritable> {
    if value.is_empty() {
        return Err(Unwritable::EmptyValue);
    }
    let mut out = String::with_capacity(value.len() + 8);
    encode_part(value, &mut out);
    Ok(out)
}

/// The line `key=value` as XUnity reads it back, or why it cannot be written.
pub fn encode_line(key: &str, value: &str) -> Result<String, Unwritable> {
    if key.is_empty() {
        return Err(Unwritable::EmptyKey);
    }
    if value.is_empty() {
        return Err(Unwritable::EmptyValue);
    }
    if key.starts_with("r:") || key.starts_with("sr:") {
        return Err(Unwritable::RegexKey);
    }
    let mut out = String::with_capacity(key.len() + value.len() + 8);
    encode_part(key, &mut out);
    // A `/` ending the key and one starting the value are apart: `=` sits between.
    out.push('=');
    encode_part(value, &mut out);
    if looks_like_directive(&out)
        && let Some(at) = out.find('#')
    {
        out.replace_range(at..=at, "\\u0023");
    }
    Ok(out)
}

/// The decoder's directive test on a raw line.
fn looks_like_directive(raw: &str) -> bool {
    let head = raw.split("//").next().unwrap_or("").trim();
    if !head.starts_with('#') {
        return false;
    }
    let mut words = head.split_whitespace();
    let first = words.next().unwrap_or("");
    words.next().is_some() && matches!(first, "#set" | "#unset" | "#enable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translators::import::xunity::decode_line;

    fn round_trip(k: &str, v: &str) {
        let line = encode_line(k, v).unwrap_or_else(|e| panic!("{k:?}={v:?}: {}", e.as_str()));
        assert!(
            !line.contains('\n') && !line.contains('\r'),
            "one line: {line:?}"
        );
        assert_eq!(
            decode_line(&line),
            Some((k.to_owned(), v.to_owned())),
            "{k:?}={v:?} wrote {line:?}"
        );
    }

    #[test]
    fn every_hard_case_reads_back() {
        let cases = [
            ("Hello.", "Привет."),
            ("a=b", "в=г"),
            ("back\\slash", "\\n is literal here"),
            ("Line one\nLine two", "Строка один\nСтрока два"),
            ("CR\r\nLF", "CR\r\nLF"),
            ("http://example.com", "см. http://example.com//x"),
            ("///", "////"),
            ("a/", "/b"),
            ("50%3D", "%3D%3d %"),
            ("#set level 1", "#enable all"),
            ("  #unset x y", "#Бокс"),
            ("#Boxing #MakingFriends", "#Бокс #Дружба"),
            ("x // y", "comment // inside"),
            ("\\u0041", "\\=\\\\"),
            ("{0} and {@nickname}", "{0} и {@nickname}"),
            (
                "<color=#ff0000>Red</color>",
                "<color=#ff0000>Красный</color>",
            ),
            (" spaced ", " с пробелами "),
            ("r", "sr"),
        ];
        for (k, v) in cases {
            round_trip(k, v);
        }
    }

    #[test]
    fn every_character_pair_reads_back() {
        // Every pair from the alphabet of the troublesome characters, as key and value.
        let alphabet = [
            '\\', '=', '/', '%', '3', 'D', '#', '\n', '\r', 'u', 'n', ' ', 'a', '/',
        ];
        for a in alphabet {
            for b in alphabet {
                for c in alphabet {
                    let s: String = [a, b, c].iter().collect();
                    if s.trim().is_empty() {
                        continue;
                    }
                    let key = format!("k{s}");
                    let value = format!("v{s}");
                    round_trip(&key, &value);
                    round_trip(&format!("{s}k"), &format!("{s}v"));
                }
            }
        }
    }

    #[test]
    fn what_cannot_be_written() {
        assert_eq!(encode_line("", "x"), Err(Unwritable::EmptyKey));
        assert_eq!(encode_line("x", ""), Err(Unwritable::EmptyValue));
        assert_eq!(encode_line("r:^a$", "x"), Err(Unwritable::RegexKey));
        assert_eq!(encode_line("sr:a", "x"), Err(Unwritable::RegexKey));
    }
}
