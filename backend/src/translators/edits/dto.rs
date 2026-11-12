//! Wire types of line editing.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::{IntoParams, ToSchema};

/// A section member as an author or a voter.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditPerson {
    pub id: String,
    pub name: String,
}

/// One displayed text of one story: where an edit was made. A text outside
/// the story (found by the search) is the place [`TEXT_PLACE`] with line and
/// part 0 and its entry's English in `editKey`.
#[derive(Debug, Clone, Serialize, Deserialize, TS, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
#[ts(export)]
pub struct EditPlace {
    pub story_id: String,
    /// 1-based line in the EN script.
    pub line: i32,
    /// The option number of a decision, else 0.
    pub part: i16,
    /// A text outside the story: the English of its entry, or `md5:` and the
    /// md5 of that English (hex) for a long one, so that a place always fits
    /// an address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub edit_key: Option<String>,
}

/// An entry with an English longer than this (bytes) is named by its md5.
pub const LONG_KEY: usize = 256;

const MD5_PREFIX: &str = "md5:";

/// The story id of a text outside the story: every edit there is for every place of its entry.
pub const TEXT_PLACE: &str = "@text";

impl EditPlace {
    /// A text outside the story (operators, items, the interface...).
    pub fn is_text(&self) -> bool {
        self.story_id == TEXT_PLACE
    }

    /// The place of a text outside the story.
    pub fn text(edit_key: &str) -> Self {
        Self {
            story_id: TEXT_PLACE.to_owned(),
            line: 0,
            part: 0,
            edit_key: Some(edit_key.to_owned()),
        }
    }

    /// The place of a text outside the story, a long one by the md5 of its
    /// English (as Postgres's `md5()` gives it).
    pub fn text_by_md5(edit_key: &str, md5: &str) -> Self {
        if edit_key.len() > LONG_KEY {
            Self::text(&format!("{MD5_PREFIX}{md5}"))
        } else {
            Self::text(edit_key)
        }
    }

    /// The md5 a key names its entry by, when it does (`md5:` and 32 hex digits).
    pub fn md5_of(key: &str) -> Option<&str> {
        key.strip_prefix(MD5_PREFIX)
            .filter(|h| h.len() == 32 && h.bytes().all(|b| b.is_ascii_hexdigit()))
    }
}

/// A place of the game data: a field of an object (this operator's second
/// skill's description). A proposal "only here" outside the story names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameHere {
    /// `operators`, `enemies`, `stages`, `items`, `archives` or `other`.
    pub section: String,
    /// The object's id in the game data (a table's name for `other`).
    pub object_id: String,
    /// The object's English name.
    #[serde(default)]
    pub object_en: Option<String>,
    /// `name`, `skill.description`, `module.effect`, ...
    pub field: String,
    /// Which one of several (skill 2), from 1; 0 when there is one.
    pub idx: i16,
}

/// A translator's proposed Russian for one dictionary entry.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Proposal {
    #[ts(type = "number")]
    pub id: i64,
    pub edit_key: String,
    pub text: String,
    /// `own`, `sova`, `ai` (or `kit`, unused for now).
    pub source: String,
    pub author: EditPerson,
    /// Unix seconds.
    #[ts(type = "number")]
    pub created_at: i64,
    /// Unix seconds of the last rewrite, null when never rewritten.
    #[ts(type = "number | null")]
    pub edited_at: Option<i64>,
    /// Texts it has had, the current one included.
    pub versions: i32,
    /// Its text before the last rewrite; null when never rewritten.
    pub previous_text: Option<String>,
    /// Who agrees with it.
    pub agreements: Vec<EditPerson>,
    pub place: EditPlace,
    /// For this place only: never exported to the dictionaries.
    pub context: bool,
    /// A one-place proposal outside the story: the place of the game data
    /// it asks a separate translation for (the pack's maintainer decides).
    pub here: Option<GameHere>,
    /// Its English is gone from the story since an import.
    pub outdated: bool,
    /// A proposal for one place approved by the lead (for that place only).
    pub approved_by: Option<EditPerson>,
    #[ts(type = "number | null")]
    pub approved_at: Option<i64>,
    /// A one-place proposal: the pack version its text went out in, if it has.
    pub released_in: Option<String>,
    /// Rejected by a translator, with the reason its author sees. A rejected
    /// proposal leaves the line; rewriting it clears the rejection.
    pub rejected_by: Option<EditPerson>,
    #[ts(type = "number | null")]
    pub rejected_at: Option<i64>,
    pub reject_reason: Option<String>,
}

/// One text a proposal has had.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProposalVersion {
    #[ts(type = "number")]
    pub id: i64,
    pub text: String,
    pub source: String,
    pub author: EditPerson,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// One step in the history of a line's translation, with the text it replaced.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistoryEvent {
    /// `game` (an import brought a new Russian from the pack), `proposal` (a
    /// proposal written or rewritten), `approve` or `withdraw` (a translator),
    /// `reject` (a proposal rejected: `text` is the reason, if any) or
    /// `reopen` (the rejection taken back).
    pub kind: String,
    /// Unix seconds.
    #[ts(type = "number")]
    pub at: i64,
    /// Who did it; null for `game`.
    pub by: Option<EditPerson>,
    /// The text after this step.
    pub text: Option<String>,
    /// The text one step back: the proposal's previous version (the game's
    /// Russian for its first), or the game's Russian before the import.
    pub before: Option<String>,
    #[ts(type = "number | null")]
    pub proposal_id: Option<i64>,
    /// A proposal's source (`own`, `sova`, `ai`), or the approved variant (`game`, `proposal`, `sova`, `ai`).
    pub source: Option<String>,
    /// A proposal for this place only.
    pub context: bool,
    /// The proposal was deleted since.
    pub deleted: bool,
}

/// The history of the translation of the entry at a place, newest first.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LineHistory {
    pub edit_key: String,
    /// The Russian the game shows now.
    pub ru_game: Option<String>,
    pub events: Vec<HistoryEvent>,
}

/// The lead's decision for an entry.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Approval {
    /// `game` (checked and kept, never exported), `proposal`, `sova` or `ai`.
    pub variant: String,
    /// The approved proposal; null for the other variants.
    #[ts(type = "number | null")]
    pub proposal_id: Option<i64>,
    pub text: String,
    pub by: EditPerson,
    #[ts(type = "number")]
    pub at: i64,
    /// The pack version this text went out in, if it has.
    pub released_in: Option<String>,
}

/// A note on an entry.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditComment {
    #[ts(type = "number")]
    pub id: i64,
    pub text: String,
    pub author: EditPerson,
    #[ts(type = "number")]
    pub created_at: i64,
    pub resolved_by: Option<EditPerson>,
    #[ts(type = "number | null")]
    pub resolved_at: Option<i64>,
    pub place: EditPlace,
    /// The fragment of the line it is about, as selected.
    pub quote: Option<String>,
    /// `remark` (a question or problem, can be resolved) or `note` (a
    /// translator's note: never resolved, always shown).
    pub kind: String,
    /// The variant a note is about: `game`, `sova`, `ai` or `proposal`; null for the line.
    pub variant: Option<String>,
    #[ts(type = "number | null")]
    pub proposal_id: Option<i64>,
    /// The variant's text when the note was written.
    pub variant_text: Option<String>,
}

/// Everything decided or proposed for one dictionary entry.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyEdits {
    pub edit_key: String,
    pub approval: Option<Approval>,
    /// Who keeps the game's Russian ("agree" with it).
    pub game_agreements: Vec<EditPerson>,
    /// Who agrees with Sova's or the AI's variant at a place.
    pub place_agreements: Vec<PlaceAgreement>,
    /// Most agreed first, then oldest first.
    pub proposals: Vec<Proposal>,
    /// Oldest first.
    pub comments: Vec<EditComment>,
}

/// An approval or proposal left at a place whose English has changed.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OutdatedEdit {
    /// `proposal` or `approval`.
    pub kind: String,
    pub line: i32,
    pub part: i16,
    /// The English it was made for.
    pub edit_key: String,
    pub text: String,
    pub author: EditPerson,
    #[ts(type = "number")]
    pub at: i64,
}

/// Write or rewrite the caller's proposal for the entry at a place.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposalInput {
    #[serde(flatten)]
    pub place: EditPlace,
    pub text: String,
    /// `own`, `sova` or `ai`.
    pub source: String,
    /// For this place only (a context translation), not for every place of the entry.
    #[serde(default)]
    pub context: bool,
    /// With `context`, for a text outside the story: the place of the game
    /// data a separate translation is asked for.
    #[serde(default)]
    pub here: Option<GameHere>,
    /// Required for a proposal for every place when the entry stands in more than one.
    #[serde(default)]
    pub confirm_many: bool,
}

/// Rewrite one's own proposal.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposalRewrite {
    pub text: String,
    #[serde(default)]
    pub confirm_many: bool,
}

/// Agree with a variant of the entry at a place, or take it back.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgreementInput {
    #[serde(flatten)]
    pub place: EditPlace,
    /// The proposal; null = the game's Russian ("keep as in the game").
    pub proposal_id: Option<i64>,
    /// `game`, `sova`, `ai` or `proposal`; when left out, `proposal` with an id, else `game`.
    #[serde(default)]
    pub variant: Option<String>,
    pub agree: bool,
}

/// A note on the entry at a place.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommentInput {
    #[serde(flatten)]
    pub place: EditPlace,
    pub text: String,
    /// The selected fragment of the line, if any.
    #[serde(default)]
    pub quote: Option<String>,
    /// `remark` (default) or `note`.
    #[serde(default)]
    pub kind: Option<String>,
    /// For a note about one variant: `game`, `sova`, `ai` or `proposal` (with `proposalId`).
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub proposal_id: Option<i64>,
}

/// Mark a comment resolved, or not.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResolveInput {
    pub resolved: bool,
}

/// One place holding the entry: a story line, or a field of a game object.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyPlace {
    /// `story`, else the game data's section (`operators`, `items`...): then
    /// the story fields are empty and the object's fields are set.
    pub section: String,
    /// The game object's id in the game data, and its English name (a table's name for `other`).
    pub object_id: Option<String>,
    pub object_en: Option<String>,
    /// `name`, `skill.description`... (as in `TextAt`).
    pub field: Option<String>,
    /// Which one of several, from 1; 0 when there is one.
    pub idx: Option<i16>,
    pub story_id: String,
    pub line: i32,
    pub part: i16,
    /// The line's 0-based order in its story (the table's № minus one).
    pub ord: i32,
    /// The line before and after it in the story: the game's Russian, else the English.
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Every place of the entry at a place.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyPlaces {
    pub edit_key: String,
    pub total: i32,
    /// Up to 500: the story's in story order, then the game data's.
    pub places: Vec<KeyPlace>,
}

/// "My edits": which proposals.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct EditsQuery {
    /// Everyone's (the lead only); else the caller's.
    #[serde(default)]
    pub all: bool,
}

/// One row of "My edits".
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditRow {
    pub proposal: Proposal,
    /// The English of the entry.
    pub en: String,
    /// The game's Russian now ("was").
    pub ru_game: Option<String>,
    /// The proposal's place's 0-based order in its story, when the line is still there.
    pub ord: Option<i32>,
    /// `proposed`, `approved`, `otherApproved`, `outdated` or `rejected`.
    pub state: String,
    /// Places holding the entry.
    pub places: i32,
}

/// "My edits".
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditRows {
    pub all: bool,
    /// Newest first, up to 2,000.
    pub rows: Vec<EditRow>,
}

/// Who a story sprite is, for the Service rows ("On stage: Amiya").
#[derive(Debug, Serialize, TS, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CharName {
    /// The sprite's id without pose and face (`char_002_amiya`).
    pub char_id: String,
    pub name_en: String,
    pub name_ru: Option<String>,
    /// Named by who talks right after it (not an operator): a guess.
    pub guessed: bool,
}

/// A chapter or story name of the library with the game's Russian.
#[derive(Debug, Serialize, TS, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NameRu {
    pub en: String,
    pub ru: String,
}

/// Who agrees with Sova's or the AI's variant at one place.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaceAgreement {
    /// `sova` or `ai`.
    pub variant: String,
    pub place: EditPlace,
    pub people: Vec<EditPerson>,
}

/// A member who has read a story, and when.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StoryRead {
    pub by: EditPerson,
    #[ts(type = "number")]
    pub at: i64,
}

/// Mark a story read by the caller, or not.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadInput {
    pub read: bool,
}

/// A story's progress: its lines that can be edited, approved ones, ones
/// waiting for a decision (proposals, not approved) and outdated ones.
#[derive(Debug, Clone, Serialize, TS, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StoryProgress {
    pub story_id: String,
    pub total: i32,
    pub approved: i32,
    pub waiting: i32,
    pub outdated: i32,
}

/// Approve a variant of the line's entry (the lead), or withdraw the approval.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalInput {
    #[serde(flatten)]
    pub place: EditPlace,
    /// `game`, `sova`, `ai` or `proposal` (with `proposalId`).
    pub variant: String,
    #[serde(default)]
    pub proposal_id: Option<i64>,
    pub approve: bool,
}

/// Several approvals at once: a whole scene's recommended variants, or the
/// game's Russian for a whole scene.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalBatch {
    pub items: Vec<ApprovalInput>,
}

/// How many approvals a batch made.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApprovalBatchResult {
    pub approved: i32,
    /// Items skipped (a line with nothing to translate, a variant it lacks).
    pub skipped: i32,
}

/// A translator's decision on proposals by id, from the Edits list.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDecisions {
    #[schema(value_type = Vec<i64>)]
    pub ids: Vec<i64>,
    /// `approve`, `reject`, `withdraw` (its approval) or `reopen` (a rejected one).
    pub decision: String,
    /// Why it is rejected; the author sees it.
    #[serde(default)]
    pub reason: Option<String>,
}
