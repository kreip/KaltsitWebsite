//! HTTP routes of line editing, under `/api/translators`. Members only;
//! one's own proposals and comments only, except what the lead may do.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Serialize;
use ts_rs::TS;
use utoipa::ToSchema;

use super::db::{self, PlaceLine};
use super::dto::{
    AgreementInput, CommentInput, EditPlace, EditRows, EditsQuery, KeyPlaces, LineHistory,
    ProposalInput, ProposalRewrite, ProposalVersion, ResolveInput,
};
use super::status::{self, CONFIRM_PLACES};
use crate::app::error::ApiError;
use crate::app::extractors::auth::MaybeAuthUser;
use crate::app::state::AppState;
use crate::translators::routes::{require_member, require_reader};

/// The id of what was created.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditCreated {
    #[ts(type = "number")]
    pub id: i64,
}

/// The line at a place, or 404; 400 when it has nothing to translate.
async fn editable_place(state: &AppState, place: &EditPlace) -> Result<PlaceLine, ApiError> {
    let line = db::place_line(&state.db, place)
        .await?
        .ok_or(ApiError::NotFound)?;
    if !status::editable(&line.edit_key) {
        return Err(ApiError::BadRequest(
            "this line has nothing to translate".into(),
        ));
    }
    Ok(line)
}

/// 409 when a proposal for every place of an entry that stands in more
/// places than [`CONFIRM_PLACES`] was not confirmed with the list in front of
/// the caller. A proposal for one place needs no confirmation.
async fn check_many(
    state: &AppState,
    key: &str,
    confirmed: bool,
    context: bool,
) -> Result<(), ApiError> {
    if confirmed || context {
        return Ok(());
    }
    let n = db::place_counts(&state.db, &[key.to_owned()])
        .await?
        .get(key)
        .copied()
        .unwrap_or(0);
    if n > CONFIRM_PLACES {
        return Err(ApiError::Conflict(format!(
            "the line stands in {n} places: confirm the edit"
        )));
    }
    Ok(())
}

/// The text as stored, or 400; also 400 when it is the game's Russian unchanged.
fn proposal_text(text: &str, ru_game: Option<&str>) -> Result<String, ApiError> {
    let text = status::clean_text(text).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "the text is empty or longer than {} characters",
            status::MAX_TEXT
        ))
    })?;
    if ru_game.and_then(status::clean_text).as_deref() == Some(text.as_str()) {
        return Err(ApiError::BadRequest(
            "the text is the game's Russian: keep it as in the game instead".into(),
        ));
    }
    Ok(text)
}

/// Write the caller's proposal for the line's dictionary entry, or rewrite
/// the one they have (a new version). With `context`: for this place only —
/// a story line, or outside the story (with `here`) a field of a game
/// object, which asks the pack's maintainer for a separate translation
/// there. Members only.
#[utoipa::path(
    post,
    path = "/translators/proposals",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = ProposalInput,
    responses(
        (status = 200, description = "The proposal's id.", body = EditCreated),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 409, response = crate::app::openapi::responses::Conflict),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn create_proposal(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<ProposalInput>,
) -> Result<Json<EditCreated>, ApiError> {
    let me = require_member(&state, &auth).await?;
    if !matches!(input.source.as_str(), "own" | "sova" | "artem" | "ai") {
        return Err(ApiError::BadRequest("source: own, sova, artem or ai".into()));
    }
    let line = editable_place(&state, &input.place).await?;
    // Outside the story the game shows one translation everywhere: "only here" must name
    // the place of the game data it asks a separate translation for.
    let here = match (input.context && input.place.is_text(), &input.here) {
        (true, Some(here)) => {
            if !db::game_place_exists(&state.db, &line.edit_key, here).await? {
                return Err(ApiError::BadRequest(
                    "the game data does not show this text at that place".into(),
                ));
            }
            Some(here)
        }
        (true, None) => {
            return Err(ApiError::BadRequest(
                "a text outside the story has one translation everywhere: name the place a separate one is needed at".into(),
            ));
        }
        (false, _) => None,
    };
    let text = proposal_text(&input.text, line.ru_game.as_deref())?;
    check_many(&state, &line.edit_key, input.confirm_many, input.context).await?;
    let id = db::upsert_proposal(
        &state.db,
        &line.edit_key,
        &input.place,
        &text,
        &input.source,
        input.context,
        here,
        me.id,
    )
    .await?;
    Ok(Json(EditCreated { id }))
}

/// Rewrite one's own proposal (a new version). Members only.
#[utoipa::path(
    put,
    path = "/translators/proposals/{id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "The proposal.")),
    request_body = ProposalRewrite,
    responses(
        (status = 204, description = "Rewritten."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 409, response = crate::app::openapi::responses::Conflict),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn rewrite_proposal(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
    Json(input): Json<ProposalRewrite>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    let (key, author, old, source, context) = db::proposal_owner(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if author != me.id {
        return Err(ApiError::Forbidden);
    }
    let ru_game = db::key_ru_game(&state.db, &key).await?;
    let text = proposal_text(&input.text, ru_game.as_deref())?;
    if text == old {
        return Ok(StatusCode::NO_CONTENT);
    }
    check_many(&state, &key, input.confirm_many, context).await?;
    db::rewrite_proposal(&state.db, id, &text, &source, me.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Delete one's own proposal (kept in the database, gone from the site),
/// with the votes for it. Members only.
#[utoipa::path(
    delete,
    path = "/translators/proposals/{id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "The proposal.")),
    responses(
        (status = 204, description = "Deleted."),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn delete_proposal(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    let (_, author, _, _, _) = db::proposal_owner(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if author != me.id {
        return Err(ApiError::Forbidden);
    }
    db::delete_proposal(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Every text a proposal has had, newest first. Members only.
#[utoipa::path(
    get,
    path = "/translators/proposals/{id}/versions",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "The proposal.")),
    responses(
        (status = 200, description = "The versions, the current one first.", body = Vec<ProposalVersion>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn proposal_versions(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ProposalVersion>>, ApiError> {
    require_member(&state, &auth).await?;
    db::proposal_owner(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(db::versions(&state.db, id).await?))
}

/// The history of the translation of the line at a place, newest first: the
/// pack's changes of the game's Russian, every text of every proposal and
/// the lead's decisions, each with the text one step back. Members only.
#[utoipa::path(
    get,
    path = "/translators/history",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(EditPlace),
    responses(
        (status = 200, description = "The steps, newest first.", body = LineHistory),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn line_history(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(place): Query<EditPlace>,
) -> Result<Json<LineHistory>, ApiError> {
    require_member(&state, &auth).await?;
    let line = db::place_line(&state.db, &place)
        .await?
        .ok_or(ApiError::NotFound)?;
    let events = db::history(&state.db, &line.edit_key, &place, line.ru_game.as_deref()).await?;
    Ok(Json(LineHistory {
        edit_key: line.edit_key,
        ru_game: line.ru_game,
        events,
    }))
}

/// Agree with a variant of the line's entry, or take it back: the game's
/// Russian ("keep as in the game"), a proposal (not one's own), or Sova's or
/// the AI's text at this place. Members only.
#[utoipa::path(
    put,
    path = "/translators/agreements",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = AgreementInput,
    responses(
        (status = 204, description = "Recorded."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn set_agreement(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<AgreementInput>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    let line = editable_place(&state, &input.place).await?;
    let variant = input.variant.clone().unwrap_or_else(|| {
        if input.proposal_id.is_some() {
            "proposal".into()
        } else {
            "game".into()
        }
    });
    let (proposal, place, text) = match variant.as_str() {
        "proposal" => {
            let id = input
                .proposal_id
                .ok_or_else(|| ApiError::BadRequest("a proposal vote needs its id".into()))?;
            let (key, author, _, _, _) = db::proposal_owner(&state.db, id)
                .await?
                .ok_or(ApiError::NotFound)?;
            if key != line.edit_key {
                return Err(ApiError::BadRequest(
                    "the proposal is for another line".into(),
                ));
            }
            if author == me.id {
                return Err(ApiError::BadRequest(
                    "one does not agree with one's own proposal".into(),
                ));
            }
            (Some(id), None, None)
        }
        "game" => {
            if line.ru_game.is_none() {
                return Err(ApiError::BadRequest(
                    "the game shows no Russian for this line".into(),
                ));
            }
            (None, None, None)
        }
        "sova" | "artem" | "ai" => {
            let text = line.variant(&variant).cloned();
            if text.is_none() {
                return Err(ApiError::BadRequest("this line has no such variant".into()));
            }
            (None, Some(&input.place), text)
        }
        _ => {
            return Err(ApiError::BadRequest(
                "variant: game, sova, ai or proposal".into(),
            ));
        }
    };
    db::set_agreement(
        &state.db,
        &db::Vote {
            key: &line.edit_key,
            variant: &variant,
            proposal,
            place,
            text: text.as_deref(),
        },
        me.id,
        input.agree,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A remark (a question or a problem) or a translator's note on the line's
/// entry, a note possibly about one variant. Members only.
#[utoipa::path(
    post,
    path = "/translators/comments",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = CommentInput,
    responses(
        (status = 200, description = "The comment's id.", body = EditCreated),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn create_comment(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<CommentInput>,
) -> Result<Json<EditCreated>, ApiError> {
    let me = require_member(&state, &auth).await?;
    let line = db::place_line(&state.db, &input.place)
        .await?
        .ok_or(ApiError::NotFound)?;
    let text = status::clean_comment(&input.text).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "the comment is empty or longer than {} characters",
            status::MAX_COMMENT
        ))
    })?;
    let quote = input
        .quote
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| q.chars().take(status::MAX_QUOTE).collect::<String>());
    let kind = input.kind.as_deref().unwrap_or("remark");
    if !matches!(kind, "remark" | "note") {
        return Err(ApiError::BadRequest("kind: remark or note".into()));
    }
    // A note may be about one variant; its text is kept as it is now.
    let variant_text = match (input.variant.as_deref(), input.proposal_id) {
        (None, _) => None,
        (Some("game"), _) => line.ru_game.clone(),
        (Some(v @ ("sova" | "artem" | "ai")), _) => line.variant(v).cloned(),
        (Some("proposal"), Some(id)) => {
            let (key, _, text, _, _) = db::proposal_owner(&state.db, id)
                .await?
                .ok_or(ApiError::NotFound)?;
            if key != line.edit_key {
                return Err(ApiError::BadRequest(
                    "the proposal is for another line".into(),
                ));
            }
            Some(text)
        }
        _ => {
            return Err(ApiError::BadRequest(
                "variant: game, sova, ai or proposal with its id".into(),
            ));
        }
    };
    if input.variant.is_some() && (kind != "note" || variant_text.is_none()) {
        return Err(ApiError::BadRequest(
            "only a note is about a variant, and the line must have it".into(),
        ));
    }
    let id = db::add_comment(
        &state.db,
        &line.edit_key,
        &input.place,
        &db::NewComment {
            text: &text,
            quote: quote.as_deref(),
            kind,
            variant: input.variant.as_deref(),
            proposal_id: input
                .proposal_id
                .filter(|_| input.variant.as_deref() == Some("proposal")),
            variant_text: variant_text.as_deref(),
        },
        me.id,
    )
    .await?;
    Ok(Json(EditCreated { id }))
}

/// Delete one's own comment. Members only.
#[utoipa::path(
    delete,
    path = "/translators/comments/{id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "The comment.")),
    responses(
        (status = 204, description = "Deleted."),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn delete_comment(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    let (author, _) = db::comment_author(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if author != me.id {
        return Err(ApiError::Forbidden);
    }
    db::delete_comment(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Mark a remark resolved or open again: its author or the lead. A
/// translator's note is never resolved (400).
#[utoipa::path(
    put,
    path = "/translators/comments/{id}/resolved",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "The comment.")),
    request_body = ResolveInput,
    responses(
        (status = 204, description = "Recorded."),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn resolve_comment(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
    Json(input): Json<ResolveInput>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    let (author, kind) = db::comment_author(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if kind == "note" {
        return Err(ApiError::BadRequest(
            "a translator's note is never resolved".into(),
        ));
    }
    if author != me.id && !me.leads() {
        return Err(ApiError::Forbidden);
    }
    db::resolve_comment(&state.db, id, input.resolved.then_some(me.id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Every story line holding the same dictionary entry as the line at a
/// place: an edit there changes them all. Members only.
#[utoipa::path(
    get,
    path = "/translators/places",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(EditPlace),
    responses(
        (status = 200, description = "The entry's places, up to 500.", body = KeyPlaces),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn places(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(place): Query<EditPlace>,
) -> Result<Json<KeyPlaces>, ApiError> {
    require_member(&state, &auth).await?;
    let line = db::place_line(&state.db, &place)
        .await?
        .ok_or(ApiError::NotFound)?;
    let total = db::place_counts(&state.db, std::slice::from_ref(&line.edit_key))
        .await?
        .get(&line.edit_key)
        .copied()
        .unwrap_or(0);
    let places = db::places(&state.db, &line.edit_key, 500).await?;
    Ok(Json(KeyPlaces {
        edit_key: line.edit_key,
        total,
        places,
    }))
}

/// "My edits": the caller's proposals, newest first; everyone's for the lead
/// with `all`. Members only.
#[utoipa::path(
    get,
    path = "/translators/edits",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(EditsQuery),
    responses(
        (status = 200, description = "The proposals with what the game shows now and where.", body = EditRows),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn edits(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<EditsQuery>,
) -> Result<Json<EditRows>, ApiError> {
    let me = require_member(&state, &auth).await?;
    if query.all && !me.leads() {
        return Err(ApiError::Forbidden);
    }
    let rows = db::edit_rows(&state.db, (!query.all).then_some(me.id)).await?;
    Ok(Json(EditRows {
        all: query.all,
        rows,
    }))
}

/// Who each story sprite is, by who talks right after it comes on stage
/// (rebuilt by the import), for the Service rows. Any signed-in reader.
#[utoipa::path(
    get,
    path = "/translators/characters",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The named sprites.", body = Vec<super::dto::CharName>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn characters(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<super::dto::CharName>>, ApiError> {
    require_reader(&state, &auth).await?;
    Ok(Json(db::char_names(&state.db).await?))
}

/// The story library's chapter and story names with the Russian the game
/// shows (ArknightsRU dictionaries), for the pickers. Any signed-in reader.
#[utoipa::path(
    get,
    path = "/translators/names",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "English name -> Russian, where the dictionaries have one.", body = Vec<super::dto::NameRu>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn names(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<super::dto::NameRu>>, ApiError> {
    require_reader(&state, &auth).await?;
    Ok(Json(db::names_ru(&state.db).await?))
}

/// The stories only the CN client has, in the library's order: the Story
/// tab's group «Только в китайской версии». Any signed-in reader.
#[utoipa::path(
    get,
    path = "/translators/cn-only",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Every story the EN client has not got yet, by event and order.", body = Vec<super::dto::CnOnlyStory>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn cn_only_stories(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<super::dto::CnOnlyStory>>, ApiError> {
    require_reader(&state, &auth).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT story_id, group_id, group_cn, group_ru, name_cn, name_ru, avg_tag, code, sort
             FROM translators.cn_only_stories ORDER BY group_id, sort, story_id",
        )
        .fetch_all(&state.db)
        .await?,
    ))
}

/// Mark a story read by the caller ("I have read this scene"), or not. Members only.
#[utoipa::path(
    put,
    path = "/translators/stories/{story_id}/read",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("story_id" = String, Path, description = "The story id.")),
    request_body = super::dto::ReadInput,
    responses(
        (status = 204, description = "Recorded."),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn set_story_read(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(story_id): Path<String>,
    Json(input): Json<super::dto::ReadInput>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    db::set_read(&state.db, &story_id, me.id, input.read).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Every story's progress: lines that can be edited, approved, waiting for a
/// decision, outdated. Any signed-in reader.
#[utoipa::path(
    get,
    path = "/translators/progress",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "One row per story with lines.", body = Vec<super::dto::StoryProgress>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn story_progress(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<super::dto::StoryProgress>>, ApiError> {
    require_reader(&state, &auth).await?;
    Ok(Json(db::progress(&state.db).await?))
}

/// Apply one approval or withdrawal for the lead. `Ok(false)` when the line
/// has nothing to approve for that variant.
async fn apply_approval(
    state: &AppState,
    me: &crate::translators::routes::Member,
    input: &super::dto::ApprovalInput,
) -> Result<bool, ApiError> {
    let line = editable_place(state, &input.place).await?;
    // A proposal for one place is approved for that place only.
    if input.variant == "proposal" {
        let id = input
            .proposal_id
            .ok_or_else(|| ApiError::BadRequest("a proposal approval needs its id".into()))?;
        let (key, _, text, _, context) = db::proposal_owner(&state.db, id)
            .await?
            .ok_or(ApiError::NotFound)?;
        if key != line.edit_key {
            return Err(ApiError::BadRequest(
                "the proposal is for another line".into(),
            ));
        }
        if context {
            db::approve_context(&state.db, &key, id, &input.place, me.id, input.approve).await?;
            return Ok(true);
        }
        if input.approve {
            db::approve_key(
                &state.db,
                &key,
                "proposal",
                Some(id),
                &text,
                &input.place,
                me.id,
            )
            .await?;
        } else {
            db::withdraw_key(&state.db, &key, &input.place, me.id).await?;
        }
        return Ok(true);
    }
    if !input.approve {
        db::withdraw_key(&state.db, &line.edit_key, &input.place, me.id).await?;
        return Ok(true);
    }
    let text = match input.variant.as_str() {
        "game" => line.ru_game.clone(),
        v @ ("sova" | "artem" | "ai") => line.variant(v).cloned(),
        _ => {
            return Err(ApiError::BadRequest(
                "variant: game, sova, ai or proposal".into(),
            ));
        }
    };
    let Some(text) = text else {
        return Ok(false);
    };
    db::approve_key(
        &state.db,
        &line.edit_key,
        &input.variant,
        None,
        &text,
        &input.place,
        me.id,
    )
    .await?;
    Ok(true)
}

/// Approve a variant of the line's entry, or withdraw the approval: the lead
/// only. The game's Russian approved means "checked, as in the game" (never
/// exported); a proposal for one place is approved for that place only.
#[utoipa::path(
    put,
    path = "/translators/approvals",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = super::dto::ApprovalInput,
    responses(
        (status = 204, description = "Recorded."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn set_approval(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<super::dto::ApprovalInput>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    if !me.leads() {
        return Err(ApiError::Forbidden);
    }
    if !apply_approval(&state, &me, &input).await? {
        return Err(ApiError::BadRequest("this line has no such variant".into()));
    }
    // An approved line counts as checked for the glossary: its numbers follow.
    super::super::glossary::recount_later(state.db.clone());
    Ok(StatusCode::NO_CONTENT)
}

/// Approve many lines at once (a scene's recommended variants, or the game's
/// Russian for a whole scene): the lead only. Lines with nothing to approve
/// are skipped and counted.
#[utoipa::path(
    post,
    path = "/translators/approvals/batch",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = super::dto::ApprovalBatch,
    responses(
        (status = 200, description = "How many were approved and skipped.", body = super::dto::ApprovalBatchResult),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn approve_batch(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<super::dto::ApprovalBatch>,
) -> Result<Json<super::dto::ApprovalBatchResult>, ApiError> {
    let me = require_member(&state, &auth).await?;
    if !me.leads() {
        return Err(ApiError::Forbidden);
    }
    if input.items.len() > 5_000 {
        return Err(ApiError::BadRequest("at most 5000 lines at once".into()));
    }
    let (mut approved, mut skipped) = (0, 0);
    for item in &input.items {
        match apply_approval(&state, &me, item).await {
            Ok(true) => approved += 1,
            Ok(false) | Err(ApiError::BadRequest(_) | ApiError::NotFound) => skipped += 1,
            Err(e) => return Err(e),
        }
    }
    super::super::glossary::recount_later(state.db.clone());
    Ok(Json(super::dto::ApprovalBatchResult { approved, skipped }))
}

/// The entries a translator may approve as in the game at once: in the
/// stories (or a game section's texts that stand in the story), a line with a
/// letter, the game's Russian, at least one variant (a translator's or the
/// machine's) and every variant at every place of the entry equal to the game's
/// text (typography aside, `status::same_text`); not approved, no live
/// proposal. One per entry, at its first story line; approved with
/// `/translators/approvals/batch` (variant `game`). Translators and admins.
#[utoipa::path(
    post,
    path = "/translators/approvals/matching",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = super::dto::MatchingInput,
    responses(
        (status = 200, description = "The entries, in story order.", body = super::dto::MatchingLines),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn matching_lines(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<super::dto::MatchingInput>,
) -> Result<Json<super::dto::MatchingLines>, ApiError> {
    type Row = (
        String,
        i32,
        i16,
        i32,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let me = require_member(&state, &auth).await?;
    if !me.leads() {
        return Err(ApiError::Forbidden);
    }
    if input.story_ids.len() > 2_000 {
        return Err(ApiError::BadRequest("at most 2000 stories".into()));
    }
    // Every story line of the entries looked at (a section's: those that stand in the story).
    let rows: Vec<Row> = sqlx::query_as(
        "WITH keys AS (
             SELECT DISTINCT edit_key FROM translators.story_lines
             WHERE server = $1 AND story_id = ANY($2) AND lettered
             UNION
             SELECT DISTINCT edit_key FROM translators.text_places
             WHERE server = $1 AND section = $3)
         SELECT s.story_id, s.line, s.part, s.ord, s.edit_key, s.en, s.ru_game, s.sova, s.ai
         FROM translators.story_lines s JOIN keys k ON k.edit_key = s.edit_key
         WHERE s.server = $1 AND s.lettered
           AND NOT EXISTS (SELECT 1 FROM translators.approvals a
                           WHERE a.edit_key = s.edit_key AND a.outdated_at IS NULL)
           AND NOT EXISTS (SELECT 1 FROM translators.proposals p
                           WHERE p.edit_key = s.edit_key AND p.deleted_at IS NULL
                             AND p.rejected_at IS NULL AND p.outdated_at IS NULL)
         ORDER BY s.edit_key, s.story_id, s.ord",
    )
    .bind(crate::translators::SERVER)
    .bind(&input.story_ids)
    .bind(input.section.as_deref().unwrap_or(""))
    .persistent(false)
    .fetch_all(&state.db)
    .await?;
    let mut lines: Vec<super::dto::MatchingLine> = Vec::new();
    let mut at = 0;
    while at < rows.len() {
        let key = &rows[at].4;
        let end = rows[at..]
            .iter()
            .position(|r| &r.4 != key)
            .map_or(rows.len(), |n| at + n);
        let group = &rows[at..end];
        at = end;
        let mut variants = 0;
        let all_equal = group.iter().all(|(_, _, _, _, _, _, ru, sova, ai)| {
            let Some(ru) = ru else { return false };
            [sova, ai].into_iter().flatten().all(|v| {
                variants += 1;
                status::same_text(v, ru)
            })
        });
        if !all_equal || variants == 0 {
            continue;
        }
        // The first place in the chosen stories, else the entry's first story line.
        let first = group
            .iter()
            .find(|r| input.story_ids.contains(&r.0))
            .unwrap_or(&group[0]);
        lines.push(super::dto::MatchingLine {
            place: EditPlace {
                story_id: first.0.clone(),
                line: first.1,
                part: first.2,
                edit_key: None,
            },
            ord: first.3,
            en: first.5.clone(),
            ru: first.6.clone().unwrap_or_default(),
        });
    }
    lines.sort_by(|a, b| {
        (a.place.story_id.as_str(), a.ord).cmp(&(b.place.story_id.as_str(), b.ord))
    });
    Ok(Json(super::dto::MatchingLines { lines }))
}

/// The longest reason of a rejection, in characters.
const MAX_REASON: usize = 1_000;

/// Decide proposals by id, from the Edits list (a translator or an admin):
/// `approve` (for its entry, or a one-place proposal for its place),
/// `reject` (its approval withdrawn first; the reason goes to its author),
/// `withdraw` its approval, or `reopen` a rejected one. Ids with nothing to do
/// (gone, outdated, decided already) are skipped and counted.
#[utoipa::path(
    post,
    path = "/translators/proposals/decisions",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = super::dto::ProposalDecisions,
    responses(
        (status = 200, description = "How many were decided and skipped.", body = super::dto::ApprovalBatchResult),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn decide_proposals(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<super::dto::ProposalDecisions>,
) -> Result<Json<super::dto::ApprovalBatchResult>, ApiError> {
    let me = require_member(&state, &auth).await?;
    if !me.leads() {
        return Err(ApiError::Forbidden);
    }
    if input.ids.len() > 5_000 {
        return Err(ApiError::BadRequest(
            "at most 5000 proposals at once".into(),
        ));
    }
    let decision = input.decision.as_str();
    if !matches!(decision, "approve" | "reject" | "withdraw" | "reopen") {
        return Err(ApiError::BadRequest(
            "decision: approve, reject, withdraw or reopen".into(),
        ));
    }
    let reason = input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    if reason.is_some_and(|r| r.chars().count() > MAX_REASON) {
        return Err(ApiError::BadRequest(format!(
            "the reason is longer than {MAX_REASON} characters"
        )));
    }
    let (mut done, mut skipped) = (0, 0);
    for &id in &input.ids {
        let Some(p) = db::decision_target(&state.db, id).await? else {
            skipped += 1;
            continue;
        };
        let place = EditPlace {
            story_id: p.story_id.clone(),
            line: p.line,
            part: p.part,
            edit_key: None,
        };
        let withdraw = async |p: &db::DecisionTarget| -> Result<(), ApiError> {
            if p.context {
                db::approve_context(&state.db, &p.edit_key, id, &place, me.id, false).await?;
            } else {
                db::withdraw_key(&state.db, &p.edit_key, &place, me.id).await?;
            }
            Ok(())
        };
        let acted = match decision {
            "approve" if !p.outdated && !p.approved => {
                if p.rejected {
                    db::set_rejected(&state.db, id, &p.edit_key, &place, false, None, me.id)
                        .await?;
                }
                if p.context {
                    db::approve_context(&state.db, &p.edit_key, id, &place, me.id, true).await?;
                } else {
                    db::approve_key(
                        &state.db,
                        &p.edit_key,
                        "proposal",
                        Some(id),
                        &p.text,
                        &place,
                        me.id,
                    )
                    .await?;
                }
                true
            }
            "reject" if !p.rejected => {
                if p.approved {
                    withdraw(&p).await?;
                }
                db::set_rejected(&state.db, id, &p.edit_key, &place, true, reason, me.id).await?;
                true
            }
            "withdraw" if p.approved => {
                withdraw(&p).await?;
                true
            }
            "reopen" if p.rejected => {
                db::set_rejected(&state.db, id, &p.edit_key, &place, false, None, me.id).await?;
                true
            }
            _ => false,
        };
        if acted {
            done += 1;
        } else {
            skipped += 1;
        }
    }
    super::super::glossary::recount_later(state.db.clone());
    Ok(Json(super::dto::ApprovalBatchResult {
        approved: done,
        skipped,
    }))
}
