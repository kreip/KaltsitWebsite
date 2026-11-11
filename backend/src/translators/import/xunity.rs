//! The ArknightsRU dictionaries as XUnity.AutoTranslator loads them.
//!
//! Line decoding follows `TextHelper.ReadTranslationLineAndDecode` and the file
//! order follows `TextTranslationCache.GetTranslationFiles` (translators/README.md,
//! "XUnity dictionary and plugin format"):
//!
//! * every `*.txt` under the folder, recursively, except `*resizer.txt` and the
//!   `_Substitutions` / `_Preprocessors` / `_Postprocessors` files;
//! * sorted by full path, DESCENDING, case-insensitive; a later file
//!   overwrites an earlier one, so `11_ui.txt` beats `12_story.txt`;
//! * one unescaped `=` per line, `//` starts a comment, `#set`-style
//!   directives and `r:` / `sr:` regex keys are not plain translations.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One translation: the value and the file (name only) that won it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub value: String,
    pub file: String,
}

/// Every plain translation, keyed by the decoded key.
#[derive(Debug, Default)]
pub struct Dictionaries {
    pub entries: HashMap<String, Entry>,
    /// File names in load order, for the import summary.
    pub files: Vec<String>,
    /// Lines that were not plain translations (comments, directives, regex
    /// keys, malformed lines), for the summary.
    pub skipped: usize,
}

const EXCLUDED: &[&str] = &[
    "_substitutions.txt",
    "_preprocessors.txt",
    "_postprocessors.txt",
];

/// The `.txt` files XUnity would load from `dir`, in load order.
pub fn load_order(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(std::io::Error::other)?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.ends_with(".txt")
            || name.ends_with("resizer.txt")
            || EXCLUDED.contains(&name.as_str())
        {
            continue;
        }
        files.push(entry.into_path());
    }
    files.sort_by_key(|p| std::cmp::Reverse(p.to_string_lossy().to_lowercase()));
    Ok(files)
}

/// Load every dictionary under `dir`, later files overwriting earlier ones.
pub fn load(dir: &Path) -> std::io::Result<Dictionaries> {
    let mut out = Dictionaries::default();
    for path in load_order(dir)? {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let content = std::fs::read_to_string(&path)?;
        for raw in content.split(['\r', '\n']) {
            if raw.is_empty() {
                continue;
            }
            match decode_line(raw) {
                Some((key, value)) if !is_regex_key(&key) => {
                    out.entries.insert(
                        key,
                        Entry {
                            value,
                            file: file.clone(),
                        },
                    );
                }
                _ => out.skipped += 1,
            }
        }
        out.files.push(file);
    }
    Ok(out)
}

fn is_regex_key(key: &str) -> bool {
    key.starts_with("r:") || key.starts_with("sr:")
}

fn is_directive(raw: &str) -> bool {
    let head = raw.split("//").next().unwrap_or("").trim();
    if !head.starts_with('#') {
        return false;
    }
    let mut words = head.split_whitespace();
    let first = words.next().unwrap_or("");
    words.next().is_some() && matches!(first, "#set" | "#unset" | "#enable")
}

/// Decode one dictionary line into `(key, value)`, or `None` when XUnity
/// would ignore it.
pub fn decode_line(raw: &str) -> Option<(String, String)> {
    if is_directive(raw) {
        return None;
    }
    let chars: Vec<char> = raw.chars().collect();
    let mut key = String::new();
    let mut value = String::new();
    let mut seen_eq = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let out = if seen_eq { &mut value } else { &mut key };
        match c {
            '\\' if i + 1 < chars.len() => {
                let n = chars[i + 1];
                match n {
                    '=' => out.push('='),
                    '\\' => out.push('\\'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    'u' => {
                        let hex: String = chars.get(i + 2..i + 6)?.iter().collect();
                        let code = u32::from_str_radix(&hex, 16).ok()?;
                        out.push(char::from_u32(code)?);
                        i += 6;
                        continue;
                    }
                    other => {
                        out.push('\\');
                        out.push(other);
                    }
                }
                i += 2;
                continue;
            }
            '%' if chars.get(i + 1..i + 3) == Some(&['3', 'D']) => {
                out.push('=');
                i += 3;
                continue;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                // A comment: valid only when the separator came first.
                if !seen_eq {
                    return None;
                }
                break;
            }
            '=' => {
                if seen_eq {
                    return None;
                }
                seen_eq = true;
            }
            _ => out.push(c),
        }
        i += 1;
    }
    if !seen_eq || key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_escapes_and_separator() {
        assert_eq!(
            decode_line(r"a\=b\nc=d\\e"),
            Some(("a=b\nc".to_owned(), "d\\e".to_owned()))
        );
        assert_eq!(
            decode_line(r"x\/y=z\t"),
            Some(("x\\/y".to_owned(), "z\\t".to_owned()))
        );
        assert_eq!(decode_line(r"A=B"), Some(("A".to_owned(), "B".to_owned())));
        assert_eq!(
            decode_line("a%3Db=c"),
            Some(("a=b".to_owned(), "c".to_owned()))
        );
    }

    #[test]
    fn rejects_what_xunity_ignores() {
        assert_eq!(decode_line("a=b=c"), None);
        assert_eq!(decode_line("// comment"), None);
        assert_eq!(decode_line("no separator"), None);
        assert_eq!(decode_line("=empty key"), None);
        assert_eq!(decode_line("empty value="), None);
        assert_eq!(decode_line("#set level 1=2"), None);
        assert_eq!(decode_line("comment // a=b"), None);
    }

    #[test]
    fn a_comment_after_the_separator_cuts_the_value() {
        assert_eq!(
            decode_line("a=b // c"),
            Some(("a".to_owned(), "b ".to_owned()))
        );
    }

    #[test]
    fn a_hash_line_that_is_not_a_directive_is_a_translation() {
        assert_eq!(
            decode_line("#Boxing #MakingFriends=#Бокс"),
            Some(("#Boxing #MakingFriends".to_owned(), "#Бокс".to_owned()))
        );
    }

    #[test]
    fn load_order_is_descending_and_later_wins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("11_ui.txt"), "Hi=Привет\n").unwrap();
        std::fs::write(dir.path().join("12_story.txt"), "Hi=Здравствуй\nYes=Да\n").unwrap();
        std::fs::write(dir.path().join("_Substitutions.txt"), "Hi=Нет\n").unwrap();
        std::fs::write(
            dir.path().join("_AutoGeneratedTranslations.txt"),
            "Yes=Ага\n",
        )
        .unwrap();
        let d = load(dir.path()).unwrap();
        assert_eq!(
            d.files,
            [
                "_AutoGeneratedTranslations.txt",
                "12_story.txt",
                "11_ui.txt"
            ]
        );
        assert_eq!(d.entries["Hi"].value, "Привет");
        assert_eq!(d.entries["Hi"].file, "11_ui.txt");
        assert_eq!(d.entries["Yes"].value, "Да");
    }
}
