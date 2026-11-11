//! Wire types of the search over every text.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::{IntoParams, ToSchema};

/// What to look for.
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct TextQuery {
    /// A part of the English or of the Russian: any case, ё = е. Empty lists everything the filters leave.
    #[serde(default)]
    pub q: String,
    /// Whole words only.
    #[serde(default)]
    pub whole: bool,
    /// `story`, `operators`, `enemies`, `stages`, `items`, `archives` or `other`.
    #[serde(default)]
    pub section: Option<String>,
    /// `unchecked` (nobody proposed or approved anything), `proposed`,
    /// `approved`, `untranslated` (no Russian) or `plugin` (the plugin's catalog holds it).
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub offset: Option<i64>,
    /// Up to 200; 50 when left out.
    #[serde(default)]
    pub limit: Option<i64>,
}

/// One place of a found text: a story line, or a field of a game object.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextAt {
    /// `story`, `operators`, `enemies`, `stages`, `items`, `archives` or `other`.
    pub section: String,
    /// A story line: the story, its line in the EN script, the option number and the line's order in the story.
    pub story_id: Option<String>,
    pub line: Option<i32>,
    pub part: Option<i16>,
    pub ord: Option<i32>,
    /// A game object: its id and English name (a table's name for `other`).
    pub object_id: Option<String>,
    pub object_en: Option<String>,
    /// `name`, `trait`, `talent.description`, `skill.name`, `module.effect`, ...
    pub field: Option<String>,
    /// Which one of several (skill 2), from 1; 0 when there is one.
    pub idx: Option<i16>,
}

/// One found text.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextHit {
    #[ts(type = "number")]
    pub id: i64,
    /// The entry an edit changes (its English).
    pub edit_key: String,
    /// The md5 of the entry's English: a long entry's place names it by this.
    #[serde(skip)]
    #[ts(skip)]
    pub key_md5: String,
    pub en: String,
    /// The Russian the game shows now.
    pub ru: Option<String>,
    /// The dictionary file holding it, or `plugin:<role>`.
    pub file: Option<String>,
    /// The plugin's catalog also holds it, under this role.
    pub plugin_role: Option<String>,
    /// The plugin's Russian when it differs.
    pub ru_plugin: Option<String>,
    pub sections: Vec<String>,
    /// Story lines and game data places holding it.
    pub places: i32,
    /// `unchecked`, `proposed` or `approved`.
    pub status: String,
    /// Its first places, the game data's before the story's.
    pub at: Vec<TextAt>,
}

/// A page of the search.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextSearch {
    /// Every text that matches.
    #[ts(type = "number")]
    pub total: i64,
    pub rows: Vec<TextHit>,
    /// How long the database took, in milliseconds.
    pub millis: u32,
    /// How many texts the list holds (as of the last import); 0 before the first import builds it.
    #[ts(type = "number")]
    pub indexed: i64,
}

/// One found text as the search screen shows it: like a story line, at the
/// place an edit is made from (its first story line, else the text itself).
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchLine {
    pub hit: TextHit,
    pub place: crate::translators::edits::dto::EditPlace,
    pub line: crate::translators::dto::TranslatorLine,
}

/// A page of the search screen.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchLines {
    /// Every text that matches.
    #[ts(type = "number")]
    pub total: i64,
    pub rows: Vec<SearchLine>,
    /// What was proposed, approved and said for the rows' entries (by `editKey`).
    pub edits: Vec<crate::translators::edits::dto::KeyEdits>,
    /// How long the search took in the database, in milliseconds.
    pub millis: u32,
    /// How many texts the list holds; 0 before the first import builds it.
    #[ts(type = "number")]
    pub indexed: i64,
}

/// Which section's objects to list.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ObjectsQuery {
    /// A section that has objects: `operators`, `enemies`, `stages`, `items` or `archives`.
    pub section: String,
}

/// Which object's texts to show.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ObjectLinesQuery {
    pub section: String,
    /// The object, as `GameObject.id` names it.
    pub object: String,
}

/// An object of a game section: an operator, an enemy, a stage or a zone, an
/// item; in the archives, an operator again (its file, records, stories and voice lines).
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameObject {
    /// What names it in the address: an operator's id in the game data (also for its archives); the
    /// English name for the rest, where the game data keeps several objects
    /// under one name (a stage and its challenge mode) and they are one card.
    pub id: String,
    /// The English name (a stage's with its code: `0-1 Collapse`).
    pub en: String,
    /// The name in the game's Russian, when the pack has one.
    pub ru: Option<String>,
    /// Its texts that can be edited, each English once.
    pub total: i32,
    /// Of them: approved (the game's Russian checked included).
    pub approved: i32,
    /// Of them: with a live proposal and not approved.
    pub waiting: i32,
    /// Other records of the game data with the same name and the same texts
    /// (their ids): they are this object too.
    pub also: Vec<String>,
    /// For a record that shares its name with a different one: what tells it
    /// apart — its first text the others lack, else its id.
    pub hint: Option<String>,
}

/// One text of an object: what of the object it is, and the text as a line
/// of the table, edited as the text itself (one translation everywhere).
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ObjectLine {
    /// `name`, `trait`, `talent.description`, `skill.name`, `module.effect`, ...
    pub field: String,
    /// Which one of several (skill 2), from 1; 0 when there is one.
    pub idx: i16,
    /// What tells it from the object's other texts at the same field, as
    /// codes: variants joined by `,`, the parts of one by `+`. `E2` elite,
    /// `L30` operator level, `P6` potential, `S7` skill level, `M3` mastery,
    /// `T2` module stage, `N1` the number in a list, `V2` the voice set, `C`
    /// challenge mode, `DE` / `DN` / `DT` the story / standard / adverse
    /// environment, `#…` a code of the game as it is, `@…` the id of the
    /// story an operator record's introduction leads to. May be empty.
    pub note: String,
    pub place: crate::translators::edits::dto::EditPlace,
    pub line: crate::translators::dto::TranslatorLine,
}

/// An object's texts in the order the game shows them.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ObjectLines {
    pub section: String,
    pub object: String,
    /// A stage's code (`0-1`): shown, never translated.
    pub code: Option<String>,
    pub rows: Vec<ObjectLine>,
    /// What was proposed, approved and said for the rows' entries (by `editKey`).
    pub edits: Vec<crate::translators::edits::dto::KeyEdits>,
}
