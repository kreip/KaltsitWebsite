//! "Suggest for the glossary": terms the members noticed while reading, for
//! the lead to decide. A list of its own on the Glossary tab; the team's
//! sheet is never written from the site.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::error::ApiError;
use crate::app::extractors::auth::MaybeAuthUser;
use crate::app::state::AppState;
use crate::translators::SERVER;
use crate::translators::edits::dto::EditPerson;
use crate::translators::edits::routes::EditCreated;
use crate::translators::routes::require_member;

/// The longest term and the longest note, in characters.
const MAX_TERM: usize = 200;
const MAX_NOTE: usize = 1_000;
/// How much of the line it was noticed in is kept.
const MAX_SOURCE: usize = 400;

/// A term suggested for the glossary.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GlossarySuggestion {
    #[ts(type = "number")]
    pub id: i64,
    pub en: String,
    pub ru: String,
    pub note: Option<String>,
    /// The story line it was noticed in, when it was one: the story and the
    /// line's 0-based order in it (null once the import no longer has the line).
    pub story_id: Option<String>,
    pub ord: Option<i32>,
    /// The English of the line it was noticed in (its start).
    pub source_en: Option<String>,
    pub author: EditPerson,
    /// Unix seconds.
    #[ts(type = "number")]
    pub created_at: i64,
    /// `open`, `accepted` or `declined`.
    pub status: String,
    /// The lead who decided.
    pub decided_by: Option<EditPerson>,
}

/// Suggest a term for the glossary.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionInput {
    pub en: String,
    pub ru: String,
    #[serde(default)]
    pub note: Option<String>,
    /// The story line it was noticed in (`storyId`, `line`, `part`), if any.
    #[serde(default)]
    pub story_id: Option<String>,
    #[serde(default)]
    pub line: Option<i32>,
    #[serde(default)]
    pub part: Option<i16>,
    /// The English of the line it was noticed in.
    #[serde(default)]
    pub source_en: Option<String>,
}

/// The lead's decision on a suggestion.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionDecision {
    /// `accepted`, `declined`, or `open` to take the decision back.
    pub status: String,
}

/// Trimmed, on one line; `None` when empty or longer than `max` characters.
fn clean(text: &str, max: usize) -> Option<String> {
    let t = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty() && t.chars().count() <= max).then_some(t)
}

#[derive(sqlx::FromRow)]
struct Row {
    id: i64,
    en: String,
    ru: String,
    note: Option<String>,
    story_id: Option<String>,
    ord: Option<i32>,
    source_en: Option<String>,
    author_id: Uuid,
    author_name: String,
    created_at: DateTime<Utc>,
    status: String,
    decided_by: Option<Uuid>,
    decider_name: Option<String>,
}

/// Every suggestion, the open ones first, then the newest.
#[utoipa::path(
    get,
    path = "/translators/glossary/suggestions",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The suggestions, open first, newest first.", body = Vec<GlossarySuggestion>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn list_suggestions(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<GlossarySuggestion>>, ApiError> {
    require_member(&state, &auth).await?;
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT g.id, g.en, g.ru, g.note, g.story_id, s.ord, g.source_en, g.author_id,
                COALESCE(NULLIF(u.nickname, ''), u.uid) AS author_name, g.created_at, g.status,
                g.decided_by, COALESCE(NULLIF(d.nickname, ''), d.uid) AS decider_name
         FROM translators.glossary_suggestions g
         JOIN users u ON u.id = g.author_id
         LEFT JOIN users d ON d.id = g.decided_by
         LEFT JOIN translators.story_lines s
             ON s.server = $1 AND s.story_id = g.story_id AND s.line = g.line AND s.part = g.part
         WHERE g.deleted_at IS NULL
         ORDER BY (g.status <> 'open'), g.created_at DESC, g.id DESC
         LIMIT 2000",
    )
    .bind(SERVER)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| GlossarySuggestion {
                id: r.id,
                en: r.en,
                ru: r.ru,
                note: r.note,
                story_id: r.story_id,
                ord: r.ord,
                source_en: r.source_en,
                author: EditPerson {
                    id: r.author_id.to_string(),
                    name: r.author_name,
                },
                created_at: r.created_at.timestamp(),
                status: r.status,
                decided_by: r.decided_by.map(|id| EditPerson {
                    id: id.to_string(),
                    name: r.decider_name.unwrap_or_default(),
                }),
            })
            .collect(),
    ))
}

/// Suggest a term for the glossary: its English, the Russian proposed and,
/// maybe, a note and the line it was noticed in. Members only.
#[utoipa::path(
    post,
    path = "/translators/glossary/suggestions",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = SuggestionInput,
    responses(
        (status = 200, description = "The suggestion's id.", body = EditCreated),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn create_suggestion(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<SuggestionInput>,
) -> Result<Json<EditCreated>, ApiError> {
    let me = require_member(&state, &auth).await?;
    let term = |text: &str, what: &str| {
        clean(text, MAX_TERM).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "{what} is empty or longer than {MAX_TERM} characters"
            ))
        })
    };
    let en = term(&input.en, "the English")?;
    let ru = term(&input.ru, "the Russian")?;
    let note = match input
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        Some(n) if n.chars().count() > MAX_NOTE => {
            return Err(ApiError::BadRequest(format!(
                "the note is longer than {MAX_NOTE} characters"
            )));
        }
        other => other.map(str::to_owned),
    };
    let source: Option<String> = input
        .source_en
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(MAX_SOURCE).collect());
    // The place is kept only as a whole.
    let place = match (&input.story_id, input.line, input.part) {
        (Some(story), Some(line), Some(part)) if !story.starts_with('@') => {
            Some((story.clone(), line, part))
        }
        _ => None,
    };
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO translators.glossary_suggestions (en, ru, note, story_id, line, part, source_en, author_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(en)
    .bind(ru)
    .bind(note)
    .bind(place.as_ref().map(|p| p.0.as_str()))
    .bind(place.as_ref().map(|p| p.1))
    .bind(place.as_ref().map(|p| p.2))
    .bind(source)
    .bind(me.id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(EditCreated { id }))
}

/// Decide on a suggestion — accepted (taken into the sheet), declined — or
/// open it again. The lead only.
#[utoipa::path(
    put,
    path = "/translators/glossary/suggestions/{id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "The suggestion.")),
    request_body = SuggestionDecision,
    responses(
        (status = 204, description = "Recorded."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn decide_suggestion(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
    Json(input): Json<SuggestionDecision>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    if !me.leads() {
        return Err(ApiError::Forbidden);
    }
    if !matches!(input.status.as_str(), "open" | "accepted" | "declined") {
        return Err(ApiError::BadRequest(
            "status: open, accepted or declined".into(),
        ));
    }
    let decided = (input.status != "open").then_some(me.id);
    let done = sqlx::query(
        "UPDATE translators.glossary_suggestions
         SET status = $2, decided_by = $3, decided_at = CASE WHEN $3::uuid IS NULL THEN NULL ELSE NOW() END
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(&input.status)
    .bind(decided)
    .execute(&state.db)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Delete one's own suggestion (the lead: anyone's).
#[utoipa::path(
    delete,
    path = "/translators/glossary/suggestions/{id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "The suggestion.")),
    responses(
        (status = 204, description = "Deleted."),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn delete_suggestion(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let me = require_member(&state, &auth).await?;
    let author: Option<Uuid> = sqlx::query_scalar(
        "SELECT author_id FROM translators.glossary_suggestions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let author = author.ok_or(ApiError::NotFound)?;
    if author != me.id && !me.leads() {
        return Err(ApiError::Forbidden);
    }
    sqlx::query("UPDATE translators.glossary_suggestions SET deleted_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_term_is_one_trimmed_line() {
        assert_eq!(
            clean("  Originium \n Arts ", 200).as_deref(),
            Some("Originium Arts")
        );
        assert_eq!(clean("   ", 200), None);
        assert_eq!(clean(&"a".repeat(201), 200), None);
    }
}
