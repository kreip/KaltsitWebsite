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
