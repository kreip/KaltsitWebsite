//! `GET /api/translators/texts`: the search over every text of the game.

use axum::Json;
use axum::extract::{Path, Query, State};

use super::db;
use super::dto::{
    GameObject, LevelsApplied, ObjectLine, ObjectLines, ObjectLinesQuery, ObjectsQuery, SearchLine,
    SearchLines, TextHit, TextQuery, TextSearch,
};
use crate::app::error::ApiError;
use crate::app::extractors::auth::MaybeAuthUser;
use crate::app::state::AppState;
use crate::translators::SERVER;
use crate::translators::dto::TranslatorLine;
use crate::translators::edits::dto::{EditPlace, GameHere, KeyEdits};
use crate::translators::edits::{self, status};
use crate::translators::routes::{for_guest, require_member, require_reader};

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
    "problems",
    "glossary",
    "brokenTag",
    "nickname",
    "noHighlight",
    "newlines",
    "longer",
    "gender",
    "mixedGender",
    "addressCn",
];

/// Search every text of the game — the story, the game data by section
/// (operators, enemies, stages, items, archives) and the rest of the pack's
/// dictionaries — by a part of its English or Russian (any case, ё = е),
/// with filters by section and state. The best answers first (the query
/// exactly, as whole words, at the start, anywhere), then the shorter English. Members only
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
const CARD_SECTIONS: &[&str] = &["operators", "enemies", "stages", "items", "archives"];

fn check_card(section: &str) -> Result<(), ApiError> {
    if CARD_SECTIONS.contains(&section) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "no objects in section: {section}"
        )))
    }
}

/// The objects of a game section — operators, enemies, stages, items, and
/// the operators once more for their archives — with their names in the
/// game's Russian and how much of each is checked.
/// Any signed-in reader.
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
    require_reader(&state, &auth).await?;
    check_card(&query.section)?;
    Ok(Json(db::objects(&state.db, SERVER, &query.section).await?))
}

/// One object's texts as lines of the story's table, in the order the game
/// shows them (an operator: name, class, trait, talents, skills, modules,
/// base skills, outfits; its archives: the file's parts, the operator
/// records, the modules' and outfits' stories, the voice lines). Each is edited as the text itself: one
/// translation wherever the English stands. Members, and the game's texts
/// only for a signed-in user outside the team.
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
    let member = require_reader(&state, &auth).await?;
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
        .map(|t| EditPlace::text_by_md5(&t.hit.edit_key, &t.hit.key_md5))
        .collect();
    let mut lines: Vec<TranslatorLine> = texts.iter().map(|t| text_line(&t.hit)).collect();
    // An operator's voice lines are the operator's words: "check the gender" knows who speaks.
    if query.section == super::gamedata::ARCHIVES {
        let gender: Option<String> =
            sqlx::query_scalar("SELECT gender FROM translators.char_genders WHERE char_id = $1")
                .bind(&query.object)
                .fetch_optional(&state.db)
                .await?;
        for (t, l) in texts.iter().zip(lines.iter_mut()) {
            if t.field == "voice.text" {
                l.speaker_gender.clone_from(&gender);
            }
        }
    }
    fill_level_counts(&state, &places, &mut lines).await?;
    let edits = edits::attach_at(&state.db, &places, &mut lines, |_| false).await?;
    let here: Vec<Option<GameHere>> = texts
        .iter()
        .map(|t| {
            Some(GameHere {
                section: query.section.clone(),
                object_id: t.object_id.clone(),
                object_en: None,
                field: t.field.clone(),
                idx: t.idx,
            })
        })
        .collect();
    settle_here(&mut lines, &places, &here, &edits);
    super::speakers::fill(&state.db, SERVER, &mut lines).await?;
    crate::translators::routes::mark_glossary(&state, &mut lines, &places, &edits).await?;
    // A reader outside the team: the game's texts only.
    let edits = if member.is_some() {
        edits
    } else {
        for_guest(&mut lines);
        Vec::new()
    };
    let rows = texts
        .into_iter()
        .zip(places)
        .zip(lines)
        .map(|((text, place), line)| ObjectLine {
            object_id: text.object_id,
            field: text.field,
            idx: text.idx,
            note: text.note,
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
        cn: hit.cn.clone(),
        speaker_gender: None,
        mixed_gender: false,
        sova: None,
        artem: None,
        ai: None,
        terms: Vec::new(),
        edit_key: hit.edit_key.clone(),
        editable: status::editable(&hit.edit_key),
        context_allowed: false,
        places: hit.places,
        status: String::new(),
        variant_glossary: Vec::new(),
        level_texts: 0,
        cn_only: false,
    }
}

/// The search screen: the objects whose name answers the query (operators,
/// enemies, items, stages), then the same search, each found text as a line of the
/// story's table — at its first story line (with Sova's and the AI's
/// variants there), else as the text itself, whose edit holds everywhere —
/// with the edits of their entries. Members, and the game's texts only for a
/// signed-in user outside the team.
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
    let member = require_reader(&state, &auth).await?;
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
                places.push(EditPlace::text_by_md5(&hit.edit_key, &hit.key_md5));
                lines.push(text_line(hit));
            }
        }
    }
    // An operator's voice line found as a text: the operator speaks ("check the gender").
    let voices: Vec<String> = hits
        .iter()
        .filter_map(|h| h.at.iter().find(|a| a.field.as_deref() == Some("voice.text")))
        .filter_map(|a| a.object_id.clone())
        .collect();
    if !voices.is_empty() {
        let genders: std::collections::HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
            "SELECT char_id, gender FROM translators.char_genders WHERE char_id = ANY($1)",
        )
        .bind(&voices)
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .collect();
        for (hit, line) in hits.iter().zip(lines.iter_mut()) {
            if line.kind == "text"
                && let Some(id) = hit
                    .at
                    .iter()
                    .find(|a| a.field.as_deref() == Some("voice.text"))
                    .and_then(|a| a.object_id.as_ref())
            {
                line.speaker_gender = genders.get(id).cloned();
            }
        }
    }
    fill_level_counts(&state, &places, &mut lines).await?;
    let edits = edits::attach_at(&state.db, &places, &mut lines, |_| false).await?;
    let here: Vec<Option<GameHere>> = hits.iter().map(game_here).collect();
    settle_here(&mut lines, &places, &here, &edits);
    super::speakers::fill(&state.db, SERVER, &mut lines).await?;
    crate::translators::routes::mark_glossary(&state, &mut lines, &places, &edits).await?;
    // A reader outside the team: the game's texts only.
    let edits = if member.is_some() {
        edits
    } else {
        for_guest(&mut lines);
        Vec::new()
    };
    // The objects named by the query stand above the first page.
    let objects = if query.offset.unwrap_or(0) == 0 {
        let section = query.section.as_deref().filter(|s| !s.is_empty());
        db::object_hits(&state.db, SERVER, &query.q, section).await?
    } else {
        Vec::new()
    };
    let rows = hits
        .into_iter()
        .zip(places)
        .zip(lines)
        .map(|((hit, place), line)| SearchLine { hit, place, line })
        .collect();
    Ok(Json(SearchLines {
        total,
        objects,
        rows,
        edits,
        millis,
        indexed,
    }))
}

/// The markup tags of the game's texts: the style spans with their colour in
/// the game, the terms with the name and the explanation the game shows (in
/// English and in the pack's Russian), each with how many texts use it; the
/// most used first. Any signed-in reader (and the internal service key).
#[utoipa::path(
    get,
    path = "/translators/tags",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Every tag, the most used first.", body = Vec<super::tags::GameTag>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn game_tags(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<super::tags::GameTag>>, ApiError> {
    if !auth.0.as_ref().is_some_and(|u| u.user_id == "service") {
        require_reader(&state, &auth).await?;
    }
    Ok(Json(db::tags(&state.db, SERVER).await?))
}

/// The overview above the search and the Game sections: how many texts the
/// filters leave, with Russian, approved, and each problem's count (texts
/// not approved) under the query and the section. Any signed-in reader.
#[utoipa::path(
    get,
    path = "/translators/texts/summary",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(TextQuery),
    responses(
        (status = 200, description = "The counts.", body = super::dto::TextSummary),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn text_summary(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<TextQuery>,
) -> Result<Json<super::dto::TextSummary>, ApiError> {
    require_reader(&state, &auth).await?;
    check(&query)?;
    Ok(Json(db::summary(&state.db, SERVER, &query).await?))
}

/// The game's place a found text is shown at (its first), as the page names it (`SearchTab.tsx::gameHere`).
fn game_here(hit: &TextHit) -> Option<GameHere> {
    let at = hit.at.first().filter(|a| a.story_id.is_none())?;
    Some(GameHere {
        section: at.section.clone(),
        object_id: at.object_id.clone()?,
        object_en: None,
        field: at.field.clone()?,
        idx: at.idx.unwrap_or(0),
    })
}

/// Outside the story every place of an entry is one `EditPlace`, so a translation approved for one
/// operator's line would read as every other's: a row is approved by a one-place proposal only when
/// the proposal names the row's own place.
fn settle_here(
    lines: &mut [TranslatorLine],
    places: &[EditPlace],
    here: &[Option<GameHere>],
    edits: &[KeyEdits],
) {
    for ((line, place), here) in lines.iter_mut().zip(places).zip(here) {
        if !place.is_text() || !line.editable || line.status != "approved" {
            continue;
        }
        let Some(e) = edits.iter().find(|e| e.edit_key == line.edit_key) else {
            continue;
        };
        let own = e
            .proposals
            .iter()
            .filter(|p| p.context && p.approved_at.is_some())
            .any(|p| match (&p.here, here) {
                (Some(a), Some(b)) => same_game_place(a, b),
                (Some(_), None) => false,
                (None, _) => true,
            });
        if e.approval.is_none() && !own {
            "proposed".clone_into(&mut line.status);
        }
    }
}

fn same_game_place(a: &GameHere, b: &GameHere) -> bool {
    (&a.section, &a.object_id, &a.field, a.idx) == (&b.section, &b.object_id, &b.field, b.idx)
}

/// How many skill level entries each text's template has, for the texts
/// outside the story (`TranslatorLine::level_texts`).
async fn fill_level_counts(
    state: &AppState,
    places: &[EditPlace],
    lines: &mut [TranslatorLine],
) -> Result<(), ApiError> {
    let keys: Vec<String> = lines
        .iter()
        .zip(places)
        .filter(|(_, p)| p.is_text())
        .map(|(l, _)| l.edit_key.clone())
        .collect();
    let counts = db::level_counts(&state.db, SERVER, &keys).await?;
    for (l, p) in lines.iter_mut().zip(places) {
        if p.is_text() {
            l.level_texts = counts.get(&l.edit_key).copied().unwrap_or(0);
        }
    }
    Ok(())
}

/// "Apply to the levels": the caller's proposal for a skill's description
/// template, with each level's numbers put in, as their proposals for the
/// level entries ("dealing 230% ATK …"); one per entry, the tags dropped
/// where the entry has none. An entry whose numbers do not fit the template,
/// or whose text would be the game's Russian, is skipped. Members only.
#[utoipa::path(
    post,
    path = "/translators/proposals/{id}/levels",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "My proposal for the template.")),
    responses(
        (status = 200, description = "How many level entries got a proposal, and how many were skipped.", body = LevelsApplied),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn apply_levels(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i64>,
) -> Result<Json<LevelsApplied>, ApiError> {
    let me = require_member(&state, &auth).await?;
    let (key, author, russian, source, context) = edits::db::proposal_owner(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if author != me.id {
        return Err(ApiError::Forbidden);
    }
    if context {
        return Err(ApiError::BadRequest(
            "a proposal for one place only cannot be applied to the levels".into(),
        ));
    }
    let template = db::text_en(&state.db, SERVER, &key)
        .await?
        .unwrap_or_else(|| key.clone());
    let entries = db::level_entries(&state.db, SERVER, &key).await?;
    let mut created = 0;
    let mut skipped = 0;
    for (entry_key, en, ru) in entries {
        let Some(text) = super::levels::apply(&template, &en, &russian)
            .and_then(|t| status::clean_text(&t))
            .filter(|t| ru.as_deref().and_then(status::clean_text).as_deref() != Some(t.as_str()))
        else {
            skipped += 1;
            continue;
        };
        let place = EditPlace::text(&entry_key);
        edits::db::upsert_proposal(
            &state.db, &entry_key, &place, &text, &source, false, None, me.id,
        )
        .await?;
        created += 1;
    }
    Ok(Json(LevelsApplied { created, skipped }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_separate_translation_is_for_its_own_operator() {
        let at = |object: &str| GameHere {
            section: "archives".to_owned(),
            object_id: object.to_owned(),
            object_en: None,
            field: "voice.text".to_owned(),
            idx: 17,
        };
        assert!(same_game_place(&at("char_469_indigo"), &at("char_469_indigo")));
        assert!(!same_game_place(&at("char_469_indigo"), &at("char_457_blitz")));
    }
}
