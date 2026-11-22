//! Who is a woman and who a man, as the game's own files say: an operator's
//! file opens with «[Gender] Female» (EN) / «【性别】女» (CN). A story speaker
//! is an operator when its EN name is the operator's, or its CN name (paired
//! with the CN script) is that of an operator the EN client has not got yet —
//! of a released one only the EN name counts: a nameless 医生 («A Doctor») is
//! not Doc. A name two operators of different genders share tells nothing.
//! Everyone else (nameless NPCs, `NAMESAKES`) has no gender here: never checked.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;

/// Nameless NPCs under an operator's EN name, told apart by the CN one: a Durin
/// (the people), a courier (a messenger).
const NAMESAKES: &[(&str, &str)] = &[("Durin", "杜林人"), ("Courier", "传令官")];

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

/// Each name's gender but for the operators to `skip`; returns the ids the table has.
fn names(
    excel: &Path,
    by_char: &HashMap<String, char>,
    skip: &HashSet<String>,
    out: &mut HashMap<String, Option<char>>,
) -> HashSet<String> {
    let mut seen_ids = HashSet::new();
    let Some(table) = read(excel, "character_table") else {
        return seen_ids;
    };
    for (id, c) in entries(&table, "Characters") {
        seen_ids.insert(id.to_owned());
        if skip.contains(id) {
            continue;
        }
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
    seen_ids
}

impl Genders {
    pub fn load(en_excel: &Path, cn_excel: Option<&Path>) -> Self {
        let mut by_char = HashMap::new();
        from_files(en_excel, &mut by_char);
        if let Some(cn) = cn_excel {
            from_files(cn, &mut by_char);
        }
        let mut by_en = HashMap::new();
        let released = names(en_excel, &by_char, &HashSet::new(), &mut by_en);
        let mut by_cn = HashMap::new();
        if let Some(cn) = cn_excel {
            names(cn, &by_char, &released, &mut by_cn);
        }
        Self {
            by_char,
            by_en,
            by_cn,
        }
    }

    /// A speaker's gender by its EN name (also written with dots after it:
    /// «Vulcan.»), else by the CN name of an operator not yet released in EN.
    pub fn speaker(&self, en: &str, cn: Option<&str>) -> Option<char> {
        let en = en.trim();
        let cn = cn.map(str::trim);
        if cn.is_some_and(|c| NAMESAKES.contains(&(en, c))) {
            return None;
        }
        match self
            .by_en
            .get(en)
            .or_else(|| self.by_en.get(en.trim_end_matches('.')))
        {
            Some(g) => *g,
            None => cn.and_then(|c| self.by_cn.get(c).copied().flatten()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_gender_the_file_states() {
        assert_eq!(
            stated("[Code Name] Amiya\n[Gender] Female\n[Race] Cautus"),
            Some('f')
        );
        assert_eq!(stated("【代号】银灰\n【性别】男\n"), Some('m'));
        assert_eq!(stated("[Gender] Unknown"), None);
        assert_eq!(stated("no file"), None);
    }

    #[test]
    fn a_nameless_namesake_is_no_operator() {
        let en = tempfile::tempdir().unwrap();
        let cn = tempfile::tempdir().unwrap();
        let file = |id: &str, g: &str| {
            format!(
                r#"{{"key":"{id}","value":{{"StoryTextAudio":[{{"Stories":[{{"StoryText":"[Gender] {g}"}}]}}]}}}}"#
            )
        };
        let name = |id: &str, n: &str| format!(r#"{{"key":"{id}","value":{{"Name":"{n}"}}}}"#);
        let write = |dir: &Path, files: &[String], chars: &[String]| {
            let book = format!(r#"{{"HandbookDict":[{}]}}"#, files.join(","));
            std::fs::write(dir.join("handbook_info_table.json"), book).unwrap();
            let table = format!(r#"{{"Characters":[{}]}}"#, chars.join(","));
            std::fs::write(dir.join("character_table.json"), table).unwrap();
        };
        write(
            en.path(),
            &[file("c1", "Male"), file("c2", "Female")],
            &[name("c1", "Doc"), name("c2", "Durin")],
        );
        write(
            cn.path(),
            &[file("c3", "Female")],
            &[name("c1", "医生"), name("c2", "杜林"), name("c3", "塔露拉")],
        );
        let g = Genders::load(en.path(), Some(cn.path()));
        assert_eq!(g.speaker("Doc", Some("医生")), Some('m'));
        assert_eq!(g.speaker("A Doctor", Some("医生")), None);
        assert_eq!(g.speaker("Doc..", None), Some('m'));
        assert_eq!(g.speaker("Durin", Some("杜林人")), None);
        assert_eq!(g.speaker("Talulah", Some("塔露拉")), Some('f'));
    }
}
