//! Who is a woman and who a man, as the game's own files say: an operator's
//! file opens with «[Gender] Female» (EN) / «【性别】女» (CN). A story speaker
//! is an operator when its EN name (or its CN name, paired with the CN script)
//! is the operator's; a name two operators of different genders share tells
//! nothing. Everyone else (nameless NPCs) has no gender here: never checked.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// `f` or `m` by operator id, and by the names a speaker goes by.
#[derive(Debug, Default)]
pub struct Genders {
    pub by_char: HashMap<String, char>,
    by_en: HashMap<String, Option<char>>,
    by_cn: HashMap<String, Option<char>>,
}

/// The gender an operator's file states.
pub fn stated(file: &str) -> Option<char> {
    for line in file.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("[Gender]") {
            return match v.trim().to_lowercase().as_str() {
                "female" => Some('f'),
                "male" => Some('m'),
                _ => None,
            };
        }
        if let Some(v) = l.strip_prefix("【性别】") {
            return match v.trim() {
                "女" | "女性" => Some('f'),
                "男" | "男性" => Some('m'),
                _ => None,
            };
        }
    }
    None
}

fn read(dir: &Path, table: &str) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(dir.join(format!("{table}.json"))).ok()?).ok()
}

fn entries<'a>(v: &'a Value, name: &str) -> impl Iterator<Item = (&'a str, &'a Value)> {
    v.get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| Some((e.get("key")?.as_str()?, e.get("value")?)))
}

/// Each operator's gender from its file; the EN tables first, the CN ones for the rest.
fn from_files(excel: &Path, out: &mut HashMap<String, char>) {
    let Some(table) = read(excel, "handbook_info_table") else {
        return;
    };
    for (id, file) in entries(&table, "HandbookDict") {
        let first = file
            .get("StoryTextAudio")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|p| p.get("Stories"))
            .and_then(Value::as_array)
            .and_then(|s| s.first())
            .and_then(|s| s.get("StoryText"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if let Some(g) = stated(first) {
            out.entry(id.to_owned()).or_insert(g);
        }
    }
}

fn names(excel: &Path, by_char: &HashMap<String, char>, out: &mut HashMap<String, Option<char>>) {
    let Some(table) = read(excel, "character_table") else {
        return;
    };
    for (id, c) in entries(&table, "Characters") {
        let (Some(name), Some(g)) = (
            c.get("Name").and_then(Value::as_str).map(str::trim),
            by_char.get(id),
        ) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        out.entry(name.to_owned())
            .and_modify(|seen| {
                if *seen != Some(*g) {
                    *seen = None;
                }
            })
            .or_insert(Some(*g));
    }
}

impl Genders {
    pub fn load(en_excel: &Path, cn_excel: Option<&Path>) -> Self {
        let mut by_char = HashMap::new();
        from_files(en_excel, &mut by_char);
        if let Some(cn) = cn_excel {
            from_files(cn, &mut by_char);
        }
        let mut by_en = HashMap::new();
        names(en_excel, &by_char, &mut by_en);
        let mut by_cn = HashMap::new();
        if let Some(cn) = cn_excel {
            names(cn, &by_char, &mut by_cn);
        }
        Self {
            by_char,
            by_en,
            by_cn,
        }
    }

    /// A speaker's gender by its EN name, else by its CN name.
    pub fn speaker(&self, en: &str, cn: Option<&str>) -> Option<char> {
        match self.by_en.get(en.trim()) {
            Some(g) => *g,
            None => cn.and_then(|c| self.by_cn.get(c.trim()).copied().flatten()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_gender_the_file_states() {
        assert_eq!(stated("[Code Name] Amiya\n[Gender] Female\n[Race] Cautus"), Some('f'));
        assert_eq!(stated("【代号】银灰\n【性别】男\n"), Some('m'));
        assert_eq!(stated("[Gender] Unknown"), None);
        assert_eq!(stated("no file"), None);
    }
}
