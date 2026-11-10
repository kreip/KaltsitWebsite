//! Wire types of the translators' section.

use serde::Serialize;
use ts_rs::TS;
use utoipa::ToSchema;

/// What the page needs to know about the caller.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TranslatorsMe {
    /// A signed-in translator: sees Sova's and the AI's texts.
    pub member: bool,
    /// The member's user id, to tell their own proposals and votes.
    pub user_id: Option<String>,
    /// `translator`, `editor` (the lead), `maintainer` (builds the pack), `admin`;
    /// several at once; empty for a non-member.
    pub roles: Vec<String>,
    /// The member's game nickname, shown for `{@nickname}`; null when unknown.
    pub nickname: Option<String>,
    /// The development sign-in is switched on for this backend.
    pub dev_login: bool,
    /// The rule that marks a line's recommended variant, the same for the
    /// whole team (the lead switches it): `B` (only when the game's text has a
    /// mark; the default) or `A` (the best variant of every line).
    pub recommend_rule: String,
}

/// The lead's choice of the "recommended" rule.
#[derive(Debug, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendRuleInput {
    /// `A` or `B`.
    pub rule: String,
}

/// One displayed text of a story with its Russian and the hints.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TranslatorLine {
    /// 1-based line in the EN script.
    pub line: u32,
    /// The option number of a decision (1-based), else 0.
    pub part: i16,
    /// `dialog`, `narration`, `subtitle`, `sticker` or `option`.
    pub kind: String,
    pub speaker_en: String,
    pub speaker_ru: Option<String>,
    pub en: String,
    /// The Russian the game shows now.
    pub ru_game: Option<String>,
    /// The Russian the game showed before the last import that changed it (a new pack), if one did.
    pub ru_game_before: Option<String>,
    /// Unix seconds of that import.
    #[ts(type = "number | null")]
    pub ru_game_changed_at: Option<i64>,
    /// `plugin:story` or the dictionary file it came from.
    pub ru_game_source: Option<String>,
    /// `exact`, `normalized`, `lines` or `plugin`.
    pub ru_game_match: Option<String>,
    /// `story` when the text is in the plugin's catalog, `ambiguous:story` when the plugin leaves it in English.
    pub plugin_role: Option<String>,
    /// The dictionary's value when the plugin's differs.
    pub ru_dict: Option<String>,
    pub speaker_cn: Option<String>,
    pub cn: Option<String>,
    /// Sova's translation: signed-in translators only.
    pub sova: Option<String>,
    /// The AI's translation: signed-in translators only.
    pub ai: Option<String>,
    /// Glossary terms found in the English, in order.
    pub terms: Vec<LineTerm>,
    /// The dictionary entry an edit of this line changes (its English).
    pub edit_key: String,
    /// False for a line with nothing to translate (only punctuation).
    pub editable: bool,
    /// A proposal for this place only is offered (every story line).
    pub context_allowed: bool,
    /// Story lines holding the same entry, this one included.
    pub places: i32,
    /// `unchecked`, `proposed`, `approved` or `outdated`.
    pub status: String,
    /// The variants whose Russian lacks a glossary term's form (a variant not
    /// listed holds them all): `game`, `sova`, `ai`, `approved`, `proposal:<id>`.
    pub variant_glossary: Vec<VariantGlossary>,
}

/// One variant of a line and the glossary terms its Russian lacks.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VariantGlossary {
    pub variant: String,
    pub missing: Vec<MissingTerm>,
}

/// A glossary term missing from a variant: as in the English line, and the glossary's Russian.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MissingTerm {
    pub en: String,
    pub ru: String,
}

/// One story's lines.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TranslatorStoryLines {
    pub story_id: String,
    /// Unix seconds of the last import, null when nothing was imported yet.
    #[ts(type = "number | null")]
    pub imported_at: Option<i64>,
    /// The lines carry Sova's and the AI's texts.
    pub hints: bool,
    pub lines: Vec<TranslatorLine>,
    /// What was proposed, approved and said for the lines' entries (by `editKey`).
    pub edits: Vec<super::edits::dto::KeyEdits>,
    /// Decisions left at this story's places for an English that has changed.
    pub outdated: Vec<super::edits::dto::OutdatedEdit>,
    /// Sova's footnotes to the story's chapter (arktr), members only like her texts.
    pub sova_notes: Vec<String>,
    /// Who has read the story ("I have read this scene"), first readers first.
    pub reads: Vec<super::edits::dto::StoryRead>,
}

/// The import's state (service key only; not used by the page).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportStatus {
    pub running: bool,
    /// Why the last run failed, while this process remembers it.
    pub error: Option<String>,
    /// Unix seconds of the last finished import.
    pub last_finished_at: Option<i64>,
    /// The last finished import's summary (`ImportSummary`).
    #[schema(value_type = Object)]
    pub last: Option<serde_json::Value>,
}

/// A glossary term found in a line's English.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LineTerm {
    pub term_id: i32,
    /// The text as it stands in the English line.
    pub matched: String,
    pub sheet: String,
    pub ru: String,
    pub ru_plural: Option<String>,
    pub qualifier: Option<String>,
    pub note: Option<String>,
    /// From the sheet copied from Sova's glossary: a variant, never marked as not found.
    pub from_sova: bool,
    /// Whether the line's Russian holds the glossary form (by stem, ё = е); null when not checked.
    pub found: Option<bool>,
}

/// One glossary entry for the Glossary tab.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GlossaryTerm {
    pub id: i32,
    pub sheet: String,
    /// 1-based row in the sheet.
    pub row: i32,
    /// The English cell as written.
    pub en: String,
    /// The English the site searches for.
    pub key: String,
    pub qualifier: Option<String>,
    pub ru: String,
    pub ru_plural: Option<String>,
    pub note: Option<String>,
    pub synonyms: Option<String>,
    pub comment: Option<String>,
    pub cn: Option<String>,
    pub case_sensitive: bool,
    pub from_sova: bool,
    /// Story lines whose English holds the key.
    pub story_lines: i32,
    /// The Russian words the game uses for it, most lines first (up to 5).
    pub game_variants: Vec<super::glossary::GameVariant>,
    /// Its lines that have a Russian to check, and of those, the lines holding the glossary's form.
    pub checked_lines: i32,
    pub found_lines: i32,
    /// Its Chinese from the game's speaker names, when it is a speaker.
    pub cn_game: Option<String>,
}

/// One story line holding a glossary term.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GlossaryLine {
    pub story_id: String,
    /// 0-based order in its story (the table's № minus one).
    pub ord: i32,
    pub en: String,
    pub ru_game: Option<String>,
    /// The Russian holds the glossary's form; null when not checked.
    pub found: Option<bool>,
}

/// A term's story lines.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GlossaryLines {
    /// Every line holding the term (with `missing`: those without the form).
    pub total: i32,
    /// Up to 200, in story order.
    pub lines: Vec<GlossaryLine>,
}

/// Which lines of a term.
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct GlossaryLinesQuery {
    /// Only the lines whose Russian lacks the glossary's form.
    #[serde(default)]
    pub missing: bool,
}

/// The whole stored glossary and its freshness.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Glossary {
    /// The Google Sheet, to open and edit it.
    pub sheet_url: String,
    /// Unix seconds of the stored copy's download, null when there is none.
    #[ts(type = "number | null")]
    pub fetched_at: Option<i64>,
    /// Unix seconds of the last attempt, successful or not.
    #[ts(type = "number | null")]
    pub attempted_at: Option<i64>,
    /// Why the last attempt failed (the stored copy is older then).
    pub last_error: Option<String>,
    /// A download is running now.
    pub refreshing: bool,
    pub terms: Vec<GlossaryTerm>,
}
