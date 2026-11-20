//! Sova's and the AI's Russian, and the CN original, from the arktr pipeline's
//! output (`data/out/stage_*.json`). Read only.
//!
//! A row's `story_id` is the script path (`StoryTxt`), `src_line` its 1-based
//! line in the CN script, and `ru` one text whose `source` says whose it is:
//! `sova*` (Sova's, `ru_sova` holding her original when errata corrected it),
//! `ai_en` / `ai_cn` (machine), `punct` (punctuation only) or `pending` (none).

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    /// The row's number in its story (the key of the user's corrections: `story#line_idx`).
    #[serde(default)]
    pub line_idx: Option<u32>,
    pub src_line: u32,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub opt: Option<u32>,
    #[serde(default)]
    pub speaker_cn: String,
    #[serde(default)]
    pub speaker_ru: String,
    #[serde(default)]
    pub cn: String,
    #[serde(default)]
    pub en: String,
    #[serde(default)]
    pub ru: String,
    #[serde(default)]
    pub source: String,
}

impl Row {
    /// Sova's text for this row, when it is hers.
    pub fn sova(&self) -> Option<&str> {
        (self.source.starts_with("sova") && !self.ru.trim().is_empty()).then_some(self.ru.as_str())
    }

    /// The user's own correction of the machine's text (arktr's `user_edit`).
    pub fn user(&self) -> Option<&str> {
        (self.source == "user_edit" && !self.ru.trim().is_empty()).then_some(self.ru.as_str())
    }

    /// The AI's text for this row, when it is the machine's.
    pub fn ai(&self) -> Option<&str> {
        (self.source.starts_with("ai_") && !self.ru.trim().is_empty()).then_some(self.ru.as_str())
    }

    /// arktr keeps escapes literal (`\n` as two characters); the site decodes them.
    pub fn en_decoded(&self) -> String {
        self.en.replace("\\n", "\n")
    }
}

#[derive(Deserialize)]
struct Stage {
    #[serde(default)]
    stories: Vec<Story>,
    /// Sova's footnotes to the chapter: a paragraph opening with a
    /// superscript number starts a note, the others continue it.
    #[serde(default)]
    footnotes: Vec<String>,
}

/// Sova's footnote paragraphs as notes: `¹ — …` opens one, a paragraph
/// without a number continues the one before (or stands alone at the start).
pub fn group_footnotes(paragraphs: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in paragraphs
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
    {
        let numbered = p.starts_with(['¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹', '⁰']);
        match out.last_mut() {
            Some(last) if !numbered => {
                last.push('\n');
                last.push_str(p);
            }
            _ => out.push(p.to_owned()),
        }
    }
    out
}

/// What arktr gives the import: rows and Sova's chapter notes, by script path;
/// the Russian names of events and stories (`data/out/story_info.json`); the
/// machine's text the user corrected (`data/review/user_edit_layer.json`).
#[derive(Default)]
pub struct Arktr {
    pub rows: HashMap<String, Vec<Row>>,
    pub notes: HashMap<String, Vec<String>>,
    /// An event's Russian name by its id (`act55side`).
    pub group_ru: HashMap<String, String>,
    /// A story's Russian name and tag by its script path.
    pub story_ru: HashMap<String, (String, String)>,
    /// The machine's text before the user's correction, by `story path#line_idx`.
    pub before_user: HashMap<String, String>,
}

#[derive(Deserialize, Default)]
struct StoryInfo {
    #[serde(default)]
    entries: HashMap<String, InfoEntry>,
    #[serde(default)]
    stories: Vec<InfoStory>,
}

#[derive(Deserialize)]
struct InfoEntry {
    #[serde(default)]
    ru: String,
}

#[derive(Deserialize)]
struct InfoStory {
    #[serde(default)]
    story_txt: String,
    #[serde(default)]
    name_ru: String,
    #[serde(default)]
    tag_ru: String,
}

#[derive(Deserialize)]
struct UserEdit {
    #[serde(default)]
    ru_before: String,
}

#[derive(Deserialize)]
struct Story {
    story_id: String,
    #[serde(default)]
    rows: Vec<Row>,
}

/// Rows and chapter notes by script path. A script listed by two stages keeps the first.
pub fn load(dir: &Path) -> anyhow::Result<Arktr> {
    let out_dir = dir.join("data/out");
    let mut paths: Vec<_> = std::fs::read_dir(&out_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("stage_") && n.ends_with(".json"))
        })
        .collect();
    paths.sort();
    let mut out = Arktr::default();
    if let Ok(bytes) = std::fs::read(out_dir.join("story_info.json"))
        && let Ok(info) = serde_json::from_slice::<StoryInfo>(&bytes)
    {
        for (id, e) in info.entries {
            if !e.ru.trim().is_empty() {
                out.group_ru.insert(id, e.ru);
            }
        }
        for st in info.stories {
            if !st.story_txt.is_empty() {
                out.story_ru.insert(st.story_txt, (st.name_ru, st.tag_ru));
            }
        }
    }
    if let Ok(bytes) = std::fs::read(dir.join("data/review/user_edit_layer.json"))
        && let Ok(layer) = serde_json::from_slice::<HashMap<String, UserEdit>>(&bytes)
    {
        for (key, e) in layer {
            if !e.ru_before.trim().is_empty() {
                out.before_user.insert(key, e.ru_before);
            }
        }
    }
    for path in paths {
        let stage: Stage = serde_json::from_slice(&std::fs::read(&path)?)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        let notes = group_footnotes(&stage.footnotes);
        for story in stage.stories {
            if !notes.is_empty() {
                out.notes
                    .entry(story.story_id.clone())
                    .or_insert_with(|| notes.clone());
            }
            out.rows.entry(story.story_id).or_insert(story.rows);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footnotes_group_by_number() {
        let p = |s: &str| s.to_owned();
        let got = group_footnotes(&[p("Intro."), p("¹ — one."), p("more."), p(""), p("² — two.")]);
        assert_eq!(got, vec!["Intro.", "¹ — one.\nmore.", "² — two."]);
    }
}
