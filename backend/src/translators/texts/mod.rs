//! Every text of the game in one list (`translators.texts`), rebuilt by the
//! import: the story lines' entries, the game data's texts by section
//! (operators, enemies, stages, items, archives), and every other entry of
//! the pack's dictionaries and the plugin's catalog (`other`). One row per
//! entry — the English an edit is keyed by — so that outside the story one
//! English has one translation wherever it stands. The search over it
//! (`GET /translators/texts`) is shared by the section's tabs.

pub mod db;
pub mod dto;
pub mod gamedata;
pub mod routes;

use std::collections::HashMap;
use std::path::Path;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::import::lines::{Line, Matcher};
use super::import::normalize::plain;
use super::import::plugin::PluginCatalog;
use super::import::xunity::Dictionaries;
use crate::app::state::AppState;
use gamedata::{GamePlace, OTHER, STORY};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(routes::search_texts))
        .routes(routes!(routes::search_lines))
}

/// One entry of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRow {
    pub edit_key: String,
    pub en: String,
    pub ru: Option<String>,
    pub file: Option<String>,
    pub plugin_role: Option<String>,
    pub ru_plugin: Option<String>,
    pub sections: Vec<&'static str>,
    pub places: u32,
}

/// A game data place of an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPlace {
    pub edit_key: String,
    pub section: &'static str,
    pub object_id: String,
    pub object_en: String,
    pub field: String,
    pub idx: i16,
    pub ord: i32,
}

/// The import's list of texts.
#[derive(Debug, Default)]
pub struct Texts {
    pub rows: Vec<TextRow>,
    pub places: Vec<TextPlace>,
}

/// What the search looks in: without tags, lower case, ё as е.
pub fn fold(s: &str) -> String {
    plain(s).to_lowercase().replace('ё', "е")
}

/// The search text of a row: its English, then its Russian.
pub fn fold_row(en: &str, ru: Option<&str>) -> String {
    match ru {
        Some(ru) => format!("{}\n{}", fold(en), fold(ru)),
        None => fold(en),
    }
}

/// The plugin's roles that hold translations of whole texts (not its
/// per-component `id:` lists, number templates or "leave in English" lists).
fn text_role(role: &str) -> bool {
    !(role.starts_with("id:")
        || role.starts_with("ambiguous:")
        || role.starts_with("number:")
        || role == "dynamic")
}

/// The catalog by plain English: `(role, Russian)`, the first role in name order.
fn plugin_index(plugin: Option<&PluginCatalog>) -> HashMap<&str, (&str, &str)> {
    let mut out: HashMap<&str, (&str, &str)> = HashMap::new();
    let Some(plugin) = plugin else {
        return out;
    };
    let mut roles: Vec<&String> = plugin.roles.keys().filter(|r| text_role(r)).collect();
    roles.sort();
    for role in roles {
        for (en, ru) in &plugin.roles[role] {
            if !ru.is_empty() {
                out.entry(en.as_str())
                    .or_insert((role.as_str(), ru.as_str()));
            }
        }
    }
    out
}

struct Builder<'a> {
    matcher: &'a Matcher<'a>,
    plugin: HashMap<&'a str, (&'a str, &'a str)>,
    rows: Vec<TextRow>,
    by_key: HashMap<String, usize>,
    places: Vec<TextPlace>,
    seen_places: std::collections::HashSet<(usize, &'static str, String, String, i16)>,
    ord: HashMap<(&'static str, String), i32>,
}

impl<'a> Builder<'a> {
    /// The entry of a game text: the dictionary's (its key, Russian and file), else the text without tags.
    fn entry(&mut self, text: &str) -> usize {
        let found = self.matcher.entry(text);
        let key = found.map_or_else(|| plain(text), |(k, _, _)| k.to_owned());
        if let Some(&i) = self.by_key.get(&key) {
            return i;
        }
        let (role, ru_plugin) = match self.plugin.get(plain(text).as_str()) {
            Some((role, ru)) => (Some((*role).to_owned()), Some((*ru).to_owned())),
            None => (None, None),
        };
        let (ru, file) = match found {
            Some((_, ru, file)) => (Some(ru.to_owned()), Some(file.to_owned())),
            // Only the plugin has it.
            None => (
                ru_plugin.clone(),
                role.as_ref().map(|r| format!("plugin:{r}")),
            ),
        };
        let ru_plugin = ru_plugin.filter(|p| Some(p) != ru.as_ref());
        self.push(TextRow {
            edit_key: key,
            en: text.to_owned(),
            ru,
            file,
            plugin_role: role,
            ru_plugin,
            sections: Vec::new(),
            places: 0,
        })
    }

    fn push(&mut self, row: TextRow) -> usize {
        let i = self.rows.len();
        self.by_key.insert(row.edit_key.clone(), i);
        self.rows.push(row);
        i
    }

    fn tag(&mut self, i: usize, section: &'static str) {
        if !self.rows[i].sections.contains(&section) {
            self.rows[i].sections.push(section);
        }
    }

    fn place(&mut self, i: usize, p: &GamePlace) {
        let id = (i, p.section, p.object_id.clone(), p.field.to_owned(), p.idx);
        if !self.seen_places.insert(id) {
            return;
        }
        let ord = self
            .ord
            .entry((p.section, p.object_id.clone()))
            .or_insert(0);
        self.places.push(TextPlace {
            edit_key: self.rows[i].edit_key.clone(),
            section: p.section,
            object_id: p.object_id.clone(),
            object_en: p.object_en.clone(),
            field: p.field.to_owned(),
            idx: p.idx,
            ord: *ord,
        });
        *ord += 1;
        self.rows[i].places += 1;
        self.tag(i, p.section);
    }
}

/// Build the list: the story's entries, the game data's places, then what
/// is left of the other tables, the dictionaries and the plugin's catalog.
pub fn collect(
    dict: &Dictionaries,
    plugin: Option<&PluginCatalog>,
    matcher: &Matcher<'_>,
    stories: &[(String, Vec<Line>)],
    excel_dir: &Path,
) -> Texts {
    let mut b = Builder {
        matcher,
        plugin: plugin_index(plugin),
        rows: Vec::new(),
        by_key: HashMap::new(),
        places: Vec::new(),
        seen_places: std::collections::HashSet::new(),
        ord: HashMap::new(),
    };
    // The story: a row per entry, as the lines were matched.
    for (_, lines) in stories {
        for l in lines {
            let i = match b.by_key.get(&l.edit_key) {
                Some(&i) => i,
                None => b.push(TextRow {
                    edit_key: l.edit_key.clone(),
                    en: l.en.clone(),
                    ru: l.ru_game.clone(),
                    file: l.ru_game_source.clone(),
                    plugin_role: l.plugin_role.clone(),
                    ru_plugin: None,
                    sections: Vec::new(),
                    places: 0,
                }),
            };
            b.rows[i].places += 1;
            b.tag(i, STORY);
        }
    }
    // The named sections of the game data.
    for p in gamedata::places(excel_dir) {
        let i = b.entry(&p.text);
        b.place(i, &p);
    }
    // The other tables: what the pack translates and no section above holds.
    let mut other: Vec<(String, String)> = Vec::new();
    gamedata::every_string(excel_dir, |table, s| {
        if matcher.entry(s).is_some() {
            other.push((table.to_owned(), s.to_owned()));
        }
    });
    for (table, s) in other {
        let known = matcher
            .entry(&s)
            .is_some_and(|(k, _, _)| b.by_key.contains_key(k));
        if known {
            continue;
        }
        let i = b.entry(&s);
        b.place(
            i,
            &GamePlace {
                section: OTHER,
                object_id: table.clone(),
                object_en: table,
                field: "text",
                idx: 0,
                text: s,
            },
        );
    }
    // The rest of the dictionaries, tied to nothing known.
    let mut left: Vec<(&String, &super::import::xunity::Entry)> = dict
        .entries
        .iter()
        .filter(|(k, _)| !b.by_key.contains_key(*k))
        .collect();
    left.sort_by(|a, c| a.0.cmp(c.0));
    for (key, e) in left {
        let (role, ru_plugin) = match b.plugin.get(plain(key).as_str()) {
            Some((role, ru)) => (Some((*role).to_owned()), Some((*ru).to_owned())),
            None => (None, None),
        };
        let i = b.push(TextRow {
            edit_key: key.clone(),
            en: key.clone(),
            ru: Some(e.value.clone()),
            file: Some(e.file.clone()),
            plugin_role: role,
            ru_plugin: ru_plugin.filter(|p| *p != e.value),
            sections: Vec::new(),
            places: 0,
        });
        b.tag(i, OTHER);
    }
    // What only the plugin's catalog holds.
    let mut only: Vec<(&str, (&str, &str))> = b
        .plugin
        .iter()
        .filter(|(en, _)| !b.by_key.contains_key(**en) && matcher.entry(en).is_none())
        .map(|(en, v)| (*en, *v))
        .collect();
    only.sort();
    for (en, (role, ru)) in only {
        let i = b.push(TextRow {
            edit_key: en.to_owned(),
            en: en.to_owned(),
            ru: Some(ru.to_owned()),
            file: Some(format!("plugin:{role}")),
            plugin_role: Some(role.to_owned()),
            ru_plugin: None,
            sections: Vec::new(),
            places: 0,
        });
        b.tag(i, OTHER);
    }
    Texts {
        rows: b.rows,
        places: b.places,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translators::import::xunity::Entry;

    fn dict(entries: &[(&str, &str, &str)]) -> Dictionaries {
        let mut d = Dictionaries::default();
        for (k, v, f) in entries {
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
    fn folds_for_the_search() {
        assert_eq!(fold("<i>Ёлка</i> and Tree"), "елка and tree");
        assert_eq!(fold_row("Amiya", Some("Амия")), "amiya\nамия");
        assert_eq!(fold_row("Amiya", None), "amiya");
    }

    #[test]
    fn one_row_per_entry_with_every_section_it_stands_in() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("character_table.json"),
            r#"{"Characters":[{"key":"char_1","value":{"Name":"Amiya","Description":"Deals <@ba.kw>Arts damage</>","Extra":"Sign in"}}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("enemy_handbook_table.json"),
            r#"{"EnemyData":[{"key":"enemy_1","value":{"Name":"Amiya","Description":"Not in the pack"}}]}"#,
        )
        .unwrap();
        let d = dict(&[
            ("Amiya", "Амия", "13_names.txt"),
            ("Deals Arts damage", "Наносит урон Искусствами", "11_ui.txt"),
            ("Sign in", "Войти", "11_ui.txt"),
            ("Left alone", "Сам по себе", "11_ui.txt"),
            ("Hello.", "Привет.", "12_story.txt"),
        ]);
        let mut plugin = PluginCatalog::default();
        plugin.roles.insert(
            "operator".to_owned(),
            HashMap::from([("Amiya".to_owned(), "Амия!".to_owned())]),
        );
        plugin.roles.insert(
            "skill".to_owned(),
            HashMap::from([("Only here".to_owned(), "Только тут".to_owned())]),
        );
        let matcher = Matcher::new(&d, Some(&plugin));
        let mut hello = crate::translators::import::lines::displayed(&[]);
        assert!(hello.is_empty());
        hello.push(Line {
            line: 1,
            part: 0,
            kind: "dialog",
            speaker_en: String::new(),
            speaker_ru: None,
            en: "Hello.".to_owned(),
            ru_game: Some("Привет.".to_owned()),
            ru_game_source: Some("12_story.txt".to_owned()),
            ru_game_match: Some("exact"),
            plugin_role: None,
            ru_dict: None,
            speaker_cn: None,
            cn: None,
            sova: None,
            ai: None,
            arktr_match: None,
            edit_key: "Hello.".to_owned(),
        });
        let stories = vec![("s1".to_owned(), hello)];
        let t = collect(&d, Some(&plugin), &matcher, &stories, dir.path());
        let row = |key: &str| t.rows.iter().find(|r| r.edit_key == key).unwrap();
        assert_eq!(row("Hello.").sections, vec![STORY]);
        // One entry for the operator and the enemy of the same name; the plugin shows its own Russian.
        let amiya = row("Amiya");
        assert_eq!(amiya.sections, vec!["operators", "enemies"]);
        assert_eq!(amiya.places, 2);
        assert_eq!(amiya.plugin_role.as_deref(), Some("operator"));
        assert_eq!(amiya.ru_plugin.as_deref(), Some("Амия!"));
        // A text with tags is the dictionary's entry without them; the game's English is kept.
        let dmg = row("Deals Arts damage");
        assert_eq!(dmg.en, "Deals <@ba.kw>Arts damage</>");
        assert_eq!(dmg.ru.as_deref(), Some("Наносит урон Искусствами"));
        // No entry in the pack: keyed by the text, no Russian.
        assert_eq!(row("Not in the pack").ru, None);
        // Another table's text the pack translates, an entry tied to nothing, a plugin-only text.
        assert_eq!(row("Sign in").sections, vec![OTHER]);
        assert_eq!(row("Sign in").places, 1);
        assert_eq!(row("Left alone").sections, vec![OTHER]);
        assert_eq!(row("Left alone").places, 0);
        assert_eq!(row("Only here").file.as_deref(), Some("plugin:skill"));
        assert_eq!(t.rows.len(), 7);
        let places: Vec<(&str, &str, &str)> = t
            .places
            .iter()
            .map(|p| (p.edit_key.as_str(), p.section, p.field.as_str()))
            .collect();
        assert_eq!(
            places,
            vec![
                ("Amiya", "operators", "name"),
                ("Deals Arts damage", "operators", "trait"),
                ("Amiya", "enemies", "name"),
                ("Not in the pack", "enemies", "description"),
                ("Sign in", OTHER, "text"),
            ]
        );
    }
}
