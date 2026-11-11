//! The texts a story shows, and what each one is matched to.

use std::collections::HashMap;

use crate::core::story::parser::StoryCommand;

use super::arktr;
use super::normalize::{for_search, plain};
use super::plugin::PluginCatalog;
use super::xunity::Dictionaries;

/// One displayed text of a story, in file order. `part` is the option number
/// of a decision (1-based), else 0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub line: u32,
    pub part: i16,
    pub kind: &'static str,
    pub speaker_en: String,
    pub speaker_ru: Option<String>,
    pub en: String,
    pub ru_game: Option<String>,
    pub ru_game_source: Option<String>,
    pub ru_game_match: Option<&'static str>,
    pub plugin_role: Option<String>,
    pub ru_dict: Option<String>,
    pub speaker_cn: Option<String>,
    pub cn: Option<String>,
    pub sova: Option<String>,
    pub ai: Option<String>,
    pub arktr_match: Option<&'static str>,
    /// The key of an edit: the dictionary entry found for the whole text,
    /// else the whole English as the plugin sees it (`plain`).
    pub edit_key: String,
}

/// Every text a reader sees, from the parsed commands, the way the book
/// (`frontend/src/lib/story/book/walk.ts`) picks them: dialogue and narration,
/// subtitles and stickers, and each option of a decision. Unlike the book it
/// keeps every branch and every repeated sticker: a translator needs them all.
pub fn displayed(commands: &[StoryCommand]) -> Vec<Line> {
    let mut out = Vec::new();
    for cmd in commands {
        let arg = |k: &str| cmd.args.get(k).map(String::as_str);
        match cmd.kind.as_str() {
            "name" | "multiline" => {
                let text = cmd.text.as_deref().unwrap_or("");
                if text.trim().is_empty() {
                    continue;
                }
                let speaker = arg("name").unwrap_or("").trim().to_owned();
                out.push(Line {
                    line: cmd.line,
                    kind: if speaker.is_empty() {
                        "narration"
                    } else {
                        "dialog"
                    },
                    speaker_en: speaker,
                    en: text.to_owned(),
                    ..Line::default()
                });
            }
            "text" | "narration" | "dialog" | "voicewithin" => {
                let text = cmd.text.as_deref().unwrap_or("");
                if text.trim().is_empty() {
                    continue;
                }
                out.push(Line {
                    line: cmd.line,
                    kind: "narration",
                    en: text.to_owned(),
                    ..Line::default()
                });
            }
            "subtitle" | "sticker" => {
                let Some(text) = arg("text").filter(|t| !t.trim().is_empty()) else {
                    continue;
                };
                out.push(Line {
                    line: cmd.line,
                    kind: if cmd.kind == "subtitle" {
                        "subtitle"
                    } else {
                        "sticker"
                    },
                    en: text.to_owned(),
                    ..Line::default()
                });
            }
            "decision" => {
                let values = arg("values").unwrap_or("").split(';').count();
                let pieces: Vec<&str> = arg("options").unwrap_or("").split(';').collect();
                // As in walk.ts: pieces past the last value belong to the last option.
                let texts: Vec<String> = if pieces.len() > values && values > 0 {
                    let mut t: Vec<String> = pieces[..values - 1]
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect();
                    t.push(pieces[values - 1..].join(";"));
                    t
                } else {
                    pieces
                        .iter()
                        .take(values)
                        .map(|s| (*s).to_owned())
                        .collect()
                };
                for (i, text) in texts.into_iter().enumerate() {
                    if text.trim().is_empty() {
                        continue;
                    }
                    out.push(Line {
                        line: cmd.line,
                        part: i16::try_from(i + 1).unwrap_or(i16::MAX),
                        kind: "option",
                        en: text,
                        ..Line::default()
                    });
                }
            }
            _ => {}
        }
    }
    out
}

/// A story sprite's id without its pose and face: `char_002_amiya_1#3` ->
/// `char_002_amiya`, `char_1002_nsabr_2` -> `char_1002_nsabr`, `avg_npc_012` stays.
pub fn char_base(id: &str) -> String {
    let id = id.trim().split('#').next().unwrap_or("").to_lowercase();
    let parts: Vec<&str> = id.split('_').collect();
    if parts.len() >= 4
        && parts
            .last()
            .is_some_and(|p| p.chars().all(|c| c.is_ascii_digit()))
    {
        parts[..parts.len() - 1].join("_")
    } else {
        id
    }
}

/// Votes for who a sprite is: after a `character` command, the first speaker
/// that talks is counted for the focused sprite (`focus=2` is `name2`).
pub fn char_votes(commands: &[StoryCommand], votes: &mut HashMap<String, HashMap<String, u32>>) {
    let mut focused: Option<String> = None;
    for cmd in commands {
        let arg = |k: &str| {
            cmd.args
                .get(k)
                .map(String::as_str)
                .filter(|v| !v.trim().is_empty())
        };
        match cmd.kind.as_str() {
            "character" | "charslot" => {
                // Every sprite on stage is listed, voted for or not.
                for n in [arg("name"), arg("name2")].into_iter().flatten() {
                    votes.entry(char_base(n)).or_default();
                }
                let pick = if arg("focus") == Some("2") {
                    arg("name2")
                } else {
                    arg("name")
                };
                focused = pick.map(char_base);
            }
            "name" | "multiline" => {
                let speaker = arg("name").unwrap_or("").trim();
                if let Some(base) = focused.take()
                    && speaker.chars().any(char::is_alphabetic)
                {
                    *votes
                        .entry(base)
                        .or_default()
                        .entry(speaker.to_owned())
                        .or_default() += 1;
                }
            }
            _ => {}
        }
    }
}

/// The dictionaries and the plugin catalog, indexed for the lookups below.
pub struct Matcher<'a> {
    dict: &'a Dictionaries,
    /// Search key -> dictionary key; on a collision the later-loaded file wins.
    by_search: HashMap<String, &'a str>,
    story_role: Option<&'a HashMap<String, String>>,
    ambiguous: Option<&'a HashMap<String, String>>,
}

impl<'a> Matcher<'a> {
    pub fn new(dict: &'a Dictionaries, plugin: Option<&'a PluginCatalog>) -> Self {
        let rank: HashMap<&str, usize> = dict
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.as_str(), i))
            .collect();
        let mut by_search: HashMap<String, &'a str> = HashMap::with_capacity(dict.entries.len());
        for (key, entry) in &dict.entries {
            let k = for_search(key);
            if k.is_empty() {
                continue;
            }
            match by_search.get(&k) {
                Some(prev)
                    if rank.get(dict.entries[*prev].file.as_str())
                        >= rank.get(entry.file.as_str()) => {}
                _ => {
                    by_search.insert(k, key.as_str());
                }
            }
        }
        Self {
            dict,
            by_search,
            story_role: plugin.and_then(|p| p.role("story")),
            ambiguous: plugin.and_then(|p| p.role("ambiguous:story")),
        }
    }

    /// One text through the dictionaries: `(key, value, file, how)`.
    fn dict_entry(&self, text: &str) -> Option<(&'a str, &'a str, &'a str, &'static str)> {
        if let Some((k, e)) = self.dict.entries.get_key_value(text) {
            return Some((k, &e.value, &e.file, "exact"));
        }
        let p = plain(text);
        if p != text
            && let Some((k, e)) = self.dict.entries.get_key_value(&p)
        {
            return Some((k, &e.value, &e.file, "exact"));
        }
        let key = self.by_search.get(&for_search(text))?;
        let e = &self.dict.entries[*key];
        Some((key, &e.value, &e.file, "normalized"))
    }

    /// The dictionary entry of a text, whole: `(key, value, file)`.
    pub fn entry(&self, text: &str) -> Option<(&'a str, &'a str, &'a str)> {
        self.dict_entry(text).map(|(k, v, f, _)| (k, v, f))
    }

    /// One text through the dictionaries: `(value, file, how)`.
    fn dict_one(&self, text: &str) -> Option<(&'a str, &'a str, &'static str)> {
        self.dict_entry(text).map(|(_, v, f, how)| (v, f, how))
    }

    /// A text through the dictionaries, whole or, failing that, line by line.
    pub fn dict(&self, text: &str) -> Option<(String, String, &'static str)> {
        if let Some((v, f, how)) = self.dict_one(text) {
            return Some((v.to_owned(), f.to_owned(), how));
        }
        let p = plain(text);
        let parts: Vec<&str> = p.split('\n').filter(|l| !l.trim().is_empty()).collect();
        if parts.len() < 2 {
            return None;
        }
        let mut values = Vec::with_capacity(parts.len());
        let mut files: Vec<&str> = Vec::new();
        for part in parts {
            let (v, f, _) = self.dict_one(part)?;
            values.push(v);
            if !files.contains(&f) {
                files.push(f);
            }
        }
        Some((values.join("\n"), files.join(", "), "lines"))
    }

    /// Fill a line's Russian "as in the game" and its speaker's name.
    pub fn apply(&self, line: &mut Line) {
        if !line.speaker_en.is_empty() {
            line.speaker_ru = self.dict(&line.speaker_en).map(|(v, _, _)| v);
        }
        let p = plain(&line.en);
        line.edit_key = self
            .dict_entry(&line.en)
            .map_or_else(|| p.clone(), |(k, _, _, _)| k.to_owned());
        let dict = self.dict(&line.en);
        if self.ambiguous.is_some_and(|m| m.contains_key(&p)) {
            // The plugin leaves these untranslated on purpose.
            line.plugin_role = Some("ambiguous:story".to_owned());
            line.ru_dict = dict.map(|(v, _, _)| v);
            return;
        }
        if let Some(v) = self
            .story_role
            .and_then(|m| m.get(&p))
            .filter(|v| !v.is_empty())
        {
            line.plugin_role = Some("story".to_owned());
            line.ru_game = Some(v.clone());
            line.ru_game_source = Some("plugin:story".to_owned());
            line.ru_game_match = Some("plugin");
            line.ru_dict = dict.map(|(d, _, _)| d).filter(|d| d != v);
            return;
        }
        if let Some((v, f, how)) = dict {
            line.ru_game = Some(v);
            line.ru_game_source = Some(f);
            line.ru_game_match = Some(how);
        }
    }
}

/// Counts of how arktr rows were attached to one story's lines.
#[derive(Debug, Default, Clone, Copy)]
pub struct ArktrCounts {
    pub by_src_line: usize,
    pub by_text: usize,
    pub unattached: usize,
}

/// Attach arktr rows to a story's lines: at `src_line` when the English there
/// is the same, else by the English text (the nearest free line), else not.
/// Returns the counts and the rows left unattached.
pub fn attach_arktr<'r>(
    lines: &mut [Line],
    rows: &'r [arktr::Row],
) -> (ArktrCounts, Vec<&'r arktr::Row>) {
    let mut counts = ArktrCounts::default();
    let mut missed = Vec::new();
    let at: HashMap<(u32, i16), usize> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| ((l.line, l.part), i))
        .collect();
    let mut by_text: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, l) in lines.iter().enumerate() {
        by_text.entry(for_search(&l.en)).or_default().push(i);
    }
    let mut taken = vec![false; lines.len()];
    for row in rows {
        let en = row.en_decoded();
        let key = for_search(&en);
        let part = if row.kind == "option" {
            row.opt.map_or(0, |o| i16::try_from(o).unwrap_or(0))
        } else {
            0
        };
        let same_place = at
            .get(&(row.src_line, part))
            .copied()
            .filter(|&i| !taken[i] && (key.is_empty() || for_search(&lines[i].en) == key));
        let (idx, how) = if let Some(i) = same_place {
            (i, "src_line")
        } else if key.is_empty() {
            counts.unattached += 1;
            missed.push(row);
            continue;
        } else if let Some(i) = by_text.get(&key).and_then(|c| {
            c.iter()
                .copied()
                .filter(|&i| !taken[i])
                .min_by_key(|&i| lines[i].line.abs_diff(row.src_line))
        }) {
            (i, "text")
        } else {
            counts.unattached += 1;
            missed.push(row);
            continue;
        };
        taken[idx] = true;
        if how == "src_line" {
            counts.by_src_line += 1;
        } else {
            counts.by_text += 1;
        }
        let l = &mut lines[idx];
        l.arktr_match = Some(how);
        let non_empty = |s: &str| (!s.trim().is_empty()).then(|| s.to_owned());
        l.cn = non_empty(&row.cn);
        l.speaker_cn = non_empty(&row.speaker_cn);
        l.sova = row.sova().map(str::to_owned);
        l.ai = row.ai().map(str::to_owned);
    }
    (counts, missed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::story::parser::parse;
    use crate::translators::import::xunity::Entry;

    fn dict(pairs: &[(&str, &str, &str)]) -> Dictionaries {
        let mut d = Dictionaries::default();
        for (k, v, f) in pairs {
            d.entries.insert(
                (*k).to_owned(),
                Entry {
                    value: (*v).to_owned(),
                    file: (*f).to_owned(),
                },
            );
            if !d.files.contains(&(*f).to_owned()) {
                d.files.push((*f).to_owned());
            }
        }
        d
    }

    #[test]
    fn sprites_are_named_by_who_talks_after_them() {
        assert_eq!(char_base("char_002_amiya_1#3"), "char_002_amiya");
        assert_eq!(char_base("char_1002_nsabr_2"), "char_1002_nsabr");
        assert_eq!(char_base("avg_npc_012"), "avg_npc_012");
        let script = "[Character(name=\"char_002_amiya_1#1\")]\n[name=\"Amiya\"] Hi.\n[name=\"Amiya\"] Again.\n[Character(name=\"char_1_a_1\", name2=\"char_2_b_1\", focus=2)]\n[name=\"B\"] Me.\n";
        let mut votes = HashMap::new();
        char_votes(&parse(script), &mut votes);
        assert_eq!(votes["char_002_amiya"]["Amiya"], 1);
        assert_eq!(votes["char_2_b"]["B"], 1);
        assert!(votes["char_1_a"].is_empty(), "on stage, never voted for");
    }

    #[test]
    fn picks_dialogue_narration_overlays_and_options() {
        let script = "[HEADER(key=\"x\")] t\n[name=\"Amiya\"] Doctor?\n[name=\"\"] Quiet.\n[dialog]\nPlain text\n[Subtitle(text=\"Day 1\", x=1, y=1)]\n[Decision(options=\"Yes;No\", values=\"1;2\")]\n";
        let lines = displayed(&parse(script));
        let got: Vec<_> = lines
            .iter()
            .map(|l| (l.line, l.part, l.kind, l.speaker_en.as_str(), l.en.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (2, 0, "dialog", "Amiya", "Doctor?"),
                (3, 0, "narration", "", "Quiet."),
                (5, 0, "narration", "", "Plain text"),
                (6, 0, "subtitle", "", "Day 1"),
                (7, 1, "option", "", "Yes"),
                (7, 2, "option", "", "No"),
            ]
        );
    }

    #[test]
    fn matches_whole_normalized_and_line_by_line() {
        let d = dict(&[
            ("Hello.", "Привет.", "11_ui.txt"),
            ("“Wait—”", "«Стой—»", "12_story.txt"),
            ("First.", "Первое.", "12_story.txt"),
            ("Second.", "Второе.", "12_story.txt"),
            ("Amiya", "Амия", "13_names.txt"),
        ]);
        let m = Matcher::new(&d, None);
        let mut l = Line {
            speaker_en: "Amiya".into(),
            en: "Hello.".into(),
            ..Line::default()
        };
        m.apply(&mut l);
        assert_eq!(l.ru_game.as_deref(), Some("Привет."));
        assert_eq!(l.ru_game_match, Some("exact"));
        assert_eq!(l.speaker_ru.as_deref(), Some("Амия"));

        let mut l = Line {
            en: "\"Wait-\"".into(),
            ..Line::default()
        };
        m.apply(&mut l);
        assert_eq!(l.ru_game_match, Some("normalized"));
        // The dictionary's own key, not the script's spelling.
        assert_eq!(l.edit_key, "“Wait—”");

        let mut l = Line {
            en: "<i>First.</i>\nSecond.".into(),
            ..Line::default()
        };
        m.apply(&mut l);
        assert_eq!(l.ru_game.as_deref(), Some("Первое.\nВторое."));
        assert_eq!(l.ru_game_match, Some("lines"));
        // Matched by parts: the edit is of the whole entry, as the plugin sees it.
        assert_eq!(l.edit_key, "First.\nSecond.");
    }

    #[test]
    fn the_plugin_catalog_wins_and_keeps_the_dictionary_value() {
        let d = dict(&[("Hi.", "Здравствуй.", "12_story.txt")]);
        let mut p = PluginCatalog::default();
        p.roles.insert(
            "story".into(),
            HashMap::from([("Hi.".to_owned(), "Привет.".to_owned())]),
        );
        p.roles.insert(
            "ambiguous:story".into(),
            HashMap::from([("Why?".to_owned(), String::new())]),
        );
        let m = Matcher::new(&d, Some(&p));
        let mut l = Line {
            en: "<i>Hi.</i>".into(),
            ..Line::default()
        };
        m.apply(&mut l);
        assert_eq!(l.ru_game.as_deref(), Some("Привет."));
        assert_eq!(l.plugin_role.as_deref(), Some("story"));
        assert_eq!(l.ru_dict.as_deref(), Some("Здравствуй."));
        let mut l = Line {
            en: "Why?".into(),
            ..Line::default()
        };
        m.apply(&mut l);
        assert_eq!(l.ru_game, None);
        assert_eq!(l.plugin_role.as_deref(), Some("ambiguous:story"));
    }

    fn row(src_line: u32, en: &str, ru: &str, source: &str) -> arktr::Row {
        arktr::Row {
            src_line,
            kind: "dialog".into(),
            opt: None,
            speaker_cn: String::new(),
            cn: format!("cn{src_line}"),
            en: en.into(),
            ru: ru.into(),
            source: source.into(),
        }
    }

    #[test]
    fn arktr_rows_attach_by_line_then_by_text() {
        let mut lines = vec![
            Line {
                line: 3,
                en: "A.".into(),
                ..Line::default()
            },
            Line {
                line: 5,
                en: "B\nC.".into(),
                ..Line::default()
            },
            Line {
                line: 9,
                en: "D.".into(),
                ..Line::default()
            },
        ];
        let rows = vec![
            row(3, "A.", "А.", "sova"),
            row(5, "B\\nC.", "Б В.", "ai_en"),
            row(8, "D.", "Д.", "sova"),
            row(20, "Gone.", "", "pending"),
        ];
        let (c, missed) = attach_arktr(&mut lines, &rows);
        assert_eq!((c.by_src_line, c.by_text, c.unattached), (2, 1, 1));
        assert_eq!(missed[0].src_line, 20);
        assert_eq!(lines[0].sova.as_deref(), Some("А."));
        assert_eq!(lines[1].ai.as_deref(), Some("Б В."));
        assert_eq!(lines[1].sova, None);
        assert_eq!(lines[2].arktr_match, Some("text"));
        assert_eq!(lines[2].cn.as_deref(), Some("cn8"));
    }
}
