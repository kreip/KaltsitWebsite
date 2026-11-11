//! `GET /api/translators/texts`: the search over every text of the game.

use axum::Json;
use axum::extract::{Query, State};

use super::db;
use super::dto::{TextQuery, TextSearch};
use crate::app::error::ApiError;
use crate::app::extractors::auth::MaybeAuthUser;
use crate::app::state::AppState;
use crate::translators::SERVER;
use crate::translators::routes::require_member;

const SECTIONS: &[&str] = &[
    "story",
    "operators",
    "enemies",
    "stages",
    "items",
    "archives",
    "other",
];
const STATUSES: &[&str] = &[
    "unchecked",
    "proposed",
    "approved",
    "untranslated",
    "plugin",
];

/// Search every text of the game — the story, the game data by section
/// (operators, enemies, stages, items, archives) and the rest of the pack's
/// dictionaries — by a part of its English or Russian (any case, ё = е),
/// with filters by section and state. Shortest English first. Members only
/// (and the internal service key, for measuring).
#[utoipa::path(
    get,
    path = "/translators/texts",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(TextQuery),
    responses(
        (status = 200, description = "A page of the matching texts, their count and the database time.", body = TextSearch),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn search_texts(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<TextQuery>,
) -> Result<Json<TextSearch>, ApiError> {
    if !auth.0.as_ref().is_some_and(|u| u.user_id == "service") {
        require_member(&state, &auth).await?;
    }
    if let Some(s) = query.section.as_deref().filter(|s| !s.is_empty())
        && !SECTIONS.contains(&s)
    {
        return Err(ApiError::BadRequest(format!("no such section: {s}")));
    }
    if let Some(s) = query.status.as_deref().filter(|s| !s.is_empty())
        && !STATUSES.contains(&s)
    {
        return Err(ApiError::BadRequest(format!("no such status: {s}")));
    }
    let started = std::time::Instant::now();
    let (total, rows) = db::search(&state.db, SERVER, &query).await?;
    let millis = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    let indexed = db::indexed_about(&state.db).await?;
    Ok(Json(TextSearch {
        total,
        rows,
        millis,
        indexed,
    }))
}
