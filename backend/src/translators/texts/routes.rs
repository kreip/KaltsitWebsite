//! `GET /api/translators/texts`: the search over every text of the game.

use axum::Json;
use axum::extract::{Query, State};

use super::db;
use super::dto::{
    GameObject, ObjectLine, ObjectLines, ObjectLinesQuery, ObjectsQuery, SearchLine, SearchLines,
    TextHit, TextQuery, TextSearch,
};
use crate::app::error::ApiError;
use crate::app::extractors::auth::MaybeAuthUser;
use crate::app::state::AppState;
use crate::translators::SERVER;
use crate::translators::dto::TranslatorLine;
use crate::translators::edits::dto::EditPlace;
use crate::translators::edits::{self, status};
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
    check(&query)?;
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

/// The sections whose texts stand at objects (a card per object).
const CARD_SECTIONS: &[&str] = &["operators", "enemies", "stages"];

fn check_card(section: &str) -> Result<(), ApiError> {
    if CARD_SECTIONS.contains(&section) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "no objects in section: {section}"
        )))
    }
}

/// The objects of a game section — operators, enemies, stages, items — with
/// their names in the game's Russian and how much of each is checked.
/// Members only.
#[utoipa::path(
    get,
    path = "/translators/objects",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(ObjectsQuery),
    responses(
        (status = 200, description = "Every object of the section.", body = Vec<GameObject>),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn objects(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<ObjectsQuery>,
) -> Result<Json<Vec<GameObject>>, ApiError> {
    require_member(&state, &auth).await?;
    check_card(&query.section)?;
    Ok(Json(db::objects(&state.db, SERVER, &query.section).await?))
}

/// One object's texts as lines of the story's table, in the order the game
/// shows them (an operator: name, class, trait, talents, skills, modules,
/// base skills, outfits). Each is edited as the text itself: one
/// translation wherever the English stands. Members only.
#[utoipa::path(
    get,
    path = "/translators/objects/lines",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(ObjectLinesQuery),
    responses(
        (status = 200, description = "The object's texts with the edits of their entries.", body = ObjectLines),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn object_lines(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<ObjectLinesQuery>,
) -> Result<Json<ObjectLines>, ApiError> {
    require_member(&state, &auth).await?;
    check_card(&query.section)?;
    let texts = db::object_texts(&state.db, SERVER, &query.section, &query.object).await?;
    if texts.is_empty() {
        return Err(ApiError::NotFound);
    }
    // A stage's code: its label without its name (a zone has neither).
    let code = (query.section == "stages" && !texts.iter().any(|t| t.field.starts_with("zone.")))
        .then(|| {
            let name = texts.iter().find(|t| t.field == "name").map(|t| &t.hit.en);
            db::stage_code(&query.object, name.map(String::as_str))
        })
        .flatten();
    let places: Vec<EditPlace> = texts
        .iter()
        .map(|t| EditPlace::text(&t.hit.edit_key))
        .collect();
    let mut lines: Vec<TranslatorLine> = texts.iter().map(|t| text_line(&t.hit)).collect();
    let edits = edits::attach_at(&state.db, &places, &mut lines, |_| false).await?;
    crate::translators::routes::mark_glossary(&state, &mut lines, &places, &edits).await?;
    let rows = texts
        .into_iter()
        .zip(places)
        .zip(lines)
        .map(|((text, place), line)| ObjectLine {
            field: text.field,
            idx: text.idx,
            place,
            line,
        })
        .collect();
    Ok(Json(ObjectLines {
        section: query.section,
        object: query.object,
        code,
        rows,
        edits,
    }))
}

/// 400 for a section or a state the list does not know.
fn check(query: &TextQuery) -> Result<(), ApiError> {
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
    Ok(())
}

/// A found text outside the story, as a line of the table.
fn text_line(hit: &TextHit) -> TranslatorLine {
    TranslatorLine {
        line: 0,
        part: 0,
        kind: "text".to_owned(),
        speaker_en: String::new(),
        speaker_ru: None,
        en: hit.en.clone(),
        ru_game: hit.ru.clone(),
        ru_game_before: None,
        ru_game_changed_at: None,
        ru_game_source: hit.file.clone(),
        ru_game_match: None,
        plugin_role: hit.plugin_role.clone(),
        ru_dict: None,
        speaker_cn: None,
        cn: None,
        sova: None,
        ai: None,
        terms: Vec::new(),
        edit_key: hit.edit_key.clone(),
        editable: status::editable(&hit.edit_key),
        context_allowed: false,
        places: hit.places,
        status: String::new(),
        variant_glossary: Vec::new(),
    }
}

/// The search screen: the same search, each found text as a line of the
/// story's table — at its first story line (with Sova's and the AI's
/// variants there), else as the text itself, whose edit holds everywhere —
/// with the edits of their entries. Members only.
#[utoipa::path(
    get,
    path = "/translators/search",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(TextQuery),
    responses(
        (status = 200, description = "A page of the matching texts as lines, their count and the database time.", body = SearchLines),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn search_lines(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<TextQuery>,
) -> Result<Json<SearchLines>, ApiError> {
    require_member(&state, &auth).await?;
    check(&query)?;
    let started = std::time::Instant::now();
    let (total, hits) = db::search(&state.db, SERVER, &query).await?;
    let millis = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    let indexed = db::indexed_about(&state.db).await?;
    let keys: Vec<String> = hits.iter().map(|h| h.edit_key.clone()).collect();
    let mut story: std::collections::HashMap<String, (String, TranslatorLine)> =
        crate::translators::db::first_story_lines(&state.db, SERVER, &keys)
            .await?
            .into_iter()
            .map(|(id, l)| (l.edit_key.clone(), (id, l)))
            .collect();
    let mut places: Vec<EditPlace> = Vec::with_capacity(hits.len());
    let mut lines: Vec<TranslatorLine> = Vec::with_capacity(hits.len());
    for hit in &hits {
        match story.remove(&hit.edit_key) {
            Some((story_id, line)) => {
                places.push(EditPlace {
                    story_id,
                    line: i32::try_from(line.line).unwrap_or(0),
                    part: line.part,
                    edit_key: None,
                });
                lines.push(line);
            }
            None => {
                places.push(EditPlace::text(&hit.edit_key));
                lines.push(text_line(hit));
            }
        }
    }
    let edits = edits::attach_at(&state.db, &places, &mut lines, |_| false).await?;
    crate::translators::routes::mark_glossary(&state, &mut lines, &places, &edits).await?;
    let rows = hits
        .into_iter()
        .zip(places)
        .zip(lines)
        .map(|((hit, place), line)| SearchLine { hit, place, line })
        .collect();
    Ok(Json(SearchLines {
        total,
        rows,
        edits,
        millis,
        indexed,
    }))
}
