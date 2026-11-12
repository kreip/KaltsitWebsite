//! HTTP routes of the translators' section, under `/api/translators`.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use uuid::Uuid;

use super::dto::{
    Glossary, GlossaryLine, GlossaryLines, GlossaryLinesQuery, GlossaryTerm, LineTerm, MissingTerm,
    RecommendRuleInput, TranslatorStoryLines, TranslatorsMe, VariantGlossary,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::dto::ImportStatus;
use super::import::{self, Sources};
use super::{SERVER, db, dev_login_enabled, edits, export, glossary};
use crate::app::error::ApiError;
use crate::app::extractors::auth::{AuthUser, MaybeAuthUser};
use crate::app::services::auth::LoginResponse;
use crate::app::services::story::cached_index;
use crate::app::state::AppState;
use crate::core::auth::jwt::create_token;
use crate::core::hypergryph::constants::Server;
use crate::database::queries::users::{create_user, find_raw_by_uid};

/// The uid of the development account. Real Arknights uids are digits only.
const DEV_UID: &str = "dev-translator";

/// A section member making a request.
pub struct Member {
    pub id: Uuid,
    /// `translator`, `editor`, `maintainer`, `admin`: one or several.
    pub roles: Vec<String>,
    pub nickname: Option<String>,
}

impl Member {
    fn has(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role || r == "admin")
    }

    /// The lead: approves, sees everyone's edits, resolves any remark.
    pub fn leads(&self) -> bool {
        self.has("editor")
    }

    /// Exports the approved entries for the game: the lead or the pack's maintainer.
    pub fn exports_for_game(&self) -> bool {
        self.has("editor") || self.has("maintainer")
    }

    /// Marks what went out in a pack version: the pack's maintainer.
    pub fn releases(&self) -> bool {
        self.has("maintainer")
    }
}

async fn member_of(state: &AppState, auth: &MaybeAuthUser) -> Result<Option<Member>, ApiError> {
    let Some(user) = &auth.0 else {
        return Ok(None);
    };
    let Ok(id) = user.user_id.parse::<Uuid>() else {
        return Ok(None);
    };
    Ok(db::member_role(&state.db, id)
        .await?
        .map(|(roles, nickname)| Member {
            id,
            roles,
            nickname,
        }))
}

/// Whether the caller is a signed-in translator, and whether the development sign-in is on.
#[utoipa::path(
    get,
    path = "/translators/me",
    tag = "translators",
    security(("bearer_auth" = []), ()),
    responses(
        (status = 200, description = "The caller's standing in the section.", body = TranslatorsMe),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn me(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<TranslatorsMe>, ApiError> {
    let member = member_of(&state, &auth).await?;
    let recommend_rule = match member {
        Some(_) => db::recommend_rule(&state.db).await?,
        None => "B".to_owned(),
    };
    Ok(Json(TranslatorsMe {
        member: member.is_some(),
        user_id: member.as_ref().map(|m| m.id.to_string()),
        nickname: member.as_ref().and_then(|m| m.nickname.clone()),
        roles: member.map(|m| m.roles).unwrap_or_default(),
        dev_login: dev_login_enabled(),
        recommend_rule,
    }))
}

/// Choose the rule that marks a line's recommended variant, for the whole
/// team: `B` (only when the game's text has a mark) or `A` (the best variant
/// of every line). The lead only.
#[utoipa::path(
    put,
    path = "/translators/settings/recommend-rule",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = RecommendRuleInput,
    responses(
        (status = 204, description = "The rule is set."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn set_recommend_rule(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<RecommendRuleInput>,
) -> Result<StatusCode, ApiError> {
    let member = require_member(&state, &auth).await?;
    if !member.leads() {
        return Err(ApiError::Forbidden);
    }
    if input.rule != "A" && input.rule != "B" {
        return Err(ApiError::BadRequest("the rule is A or B".to_owned()));
    }
    db::set_recommend_rule(&state.db, &input.rule, member.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The caller as a section member, or 403 (401 when signed out). The
/// section's data is for members only.
pub async fn require_member(state: &AppState, auth: &MaybeAuthUser) -> Result<Member, ApiError> {
    if auth.0.is_none() {
        return Err(ApiError::Unauthorized);
    }
    member_of(state, auth).await?.ok_or(ApiError::Forbidden)
}

/// One story's lines: English, the Russian the game shows now, Chinese,
/// Sova's and the AI's texts, and the glossary terms in each line. Members only.
#[utoipa::path(
    get,
    path = "/translators/stories/{story_id}/lines",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("story_id" = String, Path, description = "The story id, as in `/story/{id}`.")),
    responses(
        (status = 200, description = "The story's lines in file order; empty when nothing was imported.", body = TranslatorStoryLines),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 429, response = crate::app::openapi::responses::RateLimited),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn story_lines(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(story_id): Path<String>,
) -> Result<Json<TranslatorStoryLines>, ApiError> {
    require_member(&state, &auth).await?;
    let mut lines = db::story_lines(&state.db, SERVER, &story_id, true).await?;
    let (edits, outdated) = edits::attach(&state.db, &story_id, &mut lines).await?;
    let places: Vec<edits::dto::EditPlace> = lines
        .iter()
        .map(|l| edits::dto::EditPlace {
            story_id: story_id.clone(),
            line: i32::try_from(l.line).unwrap_or(0),
            part: l.part,
            edit_key: None,
        })
        .collect();
    mark_glossary(&state, &mut lines, &places, &edits).await?;
    let sova_notes = edits::db::sova_notes(&state.db, &story_id).await?;
    let reads = edits::db::story_reads(&state.db, &story_id).await?;
    let imported_at = db::last_import(&state.db, SERVER)
        .await?
        .map(|t| t.timestamp());
    Ok(Json(TranslatorStoryLines {
        story_id,
        imported_at,
        hints: true,
        lines,
        edits,
        outdated,
        sova_notes,
        reads,
    }))
}

/// The glossary's terms in each line's English, whether the game's Russian
/// holds their form, and which variants (the game's, Sova's, the AI's, the
/// approved text, the proposals that show at the line's place) lack one.
pub async fn mark_glossary(
    state: &AppState,
    lines: &mut [super::dto::TranslatorLine],
    places: &[edits::dto::EditPlace],
    edits: &[edits::dto::KeyEdits],
) -> Result<(), ApiError> {
    let stored = glossary::load(&state.db).await?;
    let terms: Vec<_> = stored.iter().map(glossary::StoredTerm::as_term).collect();
    // Per line: the checked terms (not Sova's sheet) by the English they matched.
    let mut checked: Vec<Vec<(String, Vec<usize>)>> = vec![Vec::new(); lines.len()];
    if !stored.is_empty() {
        let finder = glossary::matcher::Finder::new(&terms);
        for (line, groups) in lines.iter_mut().zip(checked.iter_mut()) {
            let mut seen = std::collections::HashSet::new();
            for found in finder.find(&line.en) {
                let matched = line.en[found.start..found.end].to_owned();
                for t in found.terms {
                    if !seen.insert(t) {
                        continue;
                    }
                    let term = &terms[t];
                    if !term.from_sova {
                        match groups.iter_mut().find(|g| g.0 == matched) {
                            Some(g) => g.1.push(t),
                            None => groups.push((matched.clone(), vec![t])),
                        }
                    }
                    let checked = !term.from_sova && line.ru_game.is_some();
                    line.terms.push(LineTerm {
                        term_id: stored[t].id,
                        matched: matched.clone(),
                        sheet: term.sheet.clone(),
                        ru: term.ru.clone(),
                        ru_plural: term.ru_plural.clone(),
                        qualifier: term.qualifier.clone(),
                        note: term.note.clone(),
                        from_sova: term.from_sova,
                        found: checked.then(|| {
                            glossary::matcher::term_found(
                                line.ru_game.as_deref().unwrap_or(""),
                                term,
                            )
                        }),
                    });
                }
            }
        }
    }
    // Every variant of every line against the line's glossary terms.
    let by_key: std::collections::HashMap<&str, &edits::dto::KeyEdits> =
        edits.iter().map(|e| (e.edit_key.as_str(), e)).collect();
    for ((line, groups), here) in lines.iter_mut().zip(&checked).zip(places) {
        if groups.is_empty() {
            continue;
        }
        let mut variants: Vec<(String, String)> = Vec::new();
        for (name, text) in [
            ("game", &line.ru_game),
            ("sova", &line.sova),
            ("ai", &line.ai),
        ] {
            if let Some(t) = text {
                variants.push((name.to_owned(), t.clone()));
            }
        }
        if let Some(e) = by_key.get(line.edit_key.as_str()) {
            if let Some(a) = &e.approval {
                variants.push(("approved".to_owned(), a.text.clone()));
            }
            for p in &e.proposals {
                if !p.context
                    || (p.place.story_id == here.story_id
                        && (p.place.line, p.place.part) == (here.line, here.part))
                {
                    variants.push((format!("proposal:{}", p.id), p.text.clone()));
                }
            }
        }
        for (variant, text) in variants {
            let missing: Vec<MissingTerm> = groups
                .iter()
                .filter(|(_, ts)| {
                    !ts.iter()
                        .any(|&t| glossary::matcher::term_found(&text, &terms[t]))
                })
                .map(|(en, ts)| MissingTerm {
                    en: en.clone(),
                    ru: terms[ts[0]].ru.clone(),
                })
                .collect();
            if !missing.is_empty() {
                line.variant_glossary
                    .push(VariantGlossary { variant, missing });
            }
        }
    }
    Ok(())
}

/// The whole team glossary as last downloaded. Members only.
#[utoipa::path(
    get,
    path = "/translators/glossary",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Every stored term, with the copy's date and the last download error.", body = Glossary),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn glossary(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Glossary>, ApiError> {
    require_member(&state, &auth).await?;
    glossary_body(&state).await.map(Json)
}

async fn glossary_body(state: &AppState) -> Result<Glossary, ApiError> {
    let meta = glossary::meta(&state.db).await?;
    let (fetched_at, attempted_at, last_error) = meta.unwrap_or((None, None, None));
    let terms = glossary::load(&state.db)
        .await?
        .into_iter()
        .map(|t| GlossaryTerm {
            id: t.id,
            sheet: t.sheet,
            row: t.row_no,
            en: t.en,
            key: t.key,
            qualifier: t.qualifier,
            ru: t.ru,
            ru_plural: t.ru_plural,
            note: t.note,
            synonyms: t.synonyms,
            comment: t.comment,
            cn: t.cn,
            case_sensitive: t.case_sensitive,
            from_sova: t.from_sova,
            story_lines: t.story_lines,
            game_variants: t.game_variants.0,
            checked_lines: t.checked_lines,
            found_lines: t.found_lines,
            cn_game: t.cn_game,
        })
        .collect();
    Ok(Glossary {
        sheet_url: glossary::SHEET_URL.to_owned(),
        fetched_at: fetched_at.map(|t| t.timestamp()),
        attempted_at: attempted_at.map(|t| t.timestamp()),
        last_error,
        refreshing: glossary::refreshing(),
        terms,
    })
}

/// The story lines holding one glossary term, with whether the Russian
/// holds its form. Members only.
#[utoipa::path(
    get,
    path = "/translators/glossary/{id}/lines",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "The term."), GlossaryLinesQuery),
    responses(
        (status = 200, description = "Up to 200 lines in story order, and how many there are.", body = GlossaryLines),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn glossary_lines(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(id): Path<i32>,
    Query(query): Query<GlossaryLinesQuery>,
) -> Result<Json<GlossaryLines>, ApiError> {
    const SHOWN: usize = 200;
    require_member(&state, &auth).await?;
    let stored = glossary::load(&state.db).await?;
    let at = stored
        .iter()
        .position(|t| t.id == id)
        .ok_or(ApiError::NotFound)?;
    // Candidates by a plain substring, then the glossary's own matching (word
    // edges, case, longer terms first) decides.
    let rows: Vec<(String, i32, String, Option<String>)> = sqlx::query_as(
        "SELECT story_id, ord, en, ru_game FROM translators.story_lines
         WHERE server = $1 AND strpos(lower(en), lower($2)) > 0 ORDER BY story_id, ord",
    )
    .bind(SERVER)
    .bind(&stored[at].key)
    .fetch_all(&state.db)
    .await?;
    let terms: Vec<_> = stored.iter().map(glossary::StoredTerm::as_term).collect();
    let finder = glossary::matcher::Finder::new(&terms);
    let term = &terms[at];
    let mut total = 0;
    let mut lines = Vec::new();
    for (story_id, ord, en, ru_game) in rows {
        if !finder.find(&en).iter().any(|f| f.terms.contains(&at)) {
            continue;
        }
        let found = (!term.from_sova)
            .then_some(ru_game.as_deref())
            .flatten()
            .map(|ru| glossary::matcher::term_found(ru, term));
        if query.missing && found != Some(false) {
            continue;
        }
        total += 1;
        if lines.len() < SHOWN {
            lines.push(GlossaryLine {
                story_id,
                ord,
                en,
                ru_game,
                found,
            });
        }
    }
    Ok(Json(GlossaryLines { total, lines }))
}

/// Download the glossary again from the team's sheet, in the background.
/// Members only. On failure the stored copy stays and `lastError` says why.
#[utoipa::path(
    post,
    path = "/translators/glossary/refresh",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The stored glossary, with `refreshing` set while the download runs.", body = Glossary),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn refresh_glossary(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Glossary>, ApiError> {
    require_member(&state, &auth).await?;
    if !glossary::refreshing() {
        let pool = state.db.clone();
        let http = state.http_client.clone();
        tokio::spawn(async move {
            let _ = glossary::refresh(&pool, &http).await;
        });
        // Let the task mark itself as running before answering.
        tokio::task::yield_now().await;
    }
    let mut body = glossary_body(&state).await?;
    body.refreshing = true;
    Ok(Json(body))
}

/// Start rebuilding the section's lines from the EN scripts, the ArknightsRU
/// dictionaries and arktr, in the background (it outlasts the request
/// timeout). Internal service key only; re-runnable. Poll `GET` for the result.
#[utoipa::path(
    post,
    path = "/translators/import",
    tag = "translators",
    security(("service_key" = [])),
    responses(
        (status = 200, description = "The import's state right after the request: running, with the previous result.", body = ImportStatus),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn run_import(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ImportStatus>, ApiError> {
    if auth.user_id != "service" {
        return Err(ApiError::Forbidden);
    }
    let sources = Sources::from_env().map_err(ApiError::BadRequest)?;
    if !IMPORT_RUNNING.swap(true, Ordering::SeqCst) {
        *lock_error() = None;
        let state2 = state.clone();
        tokio::spawn(async move {
            let result = import_now(&state2, sources).await;
            if let Err(e) = &result {
                // The cause too: an internal error's own text says nothing.
                tracing::error!(error = %e, cause = ?e, "translators import failed");
                *lock_error() = Some(e.to_string());
            }
            IMPORT_RUNNING.store(false, Ordering::SeqCst);
        });
    }
    import_status(State(state), auth).await
}

/// Whether an import is running, the last error, and the last finished import's summary.
#[utoipa::path(
    get,
    path = "/translators/import",
    tag = "translators",
    security(("service_key" = [])),
    responses(
        (status = 200, description = "The import's state.", body = ImportStatus),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn import_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ImportStatus>, ApiError> {
    if auth.user_id != "service" {
        return Err(ApiError::Forbidden);
    }
    let last = db::last_import_summary(&state.db, SERVER).await?;
    Ok(Json(ImportStatus {
        running: IMPORT_RUNNING.load(Ordering::SeqCst),
        error: lock_error().clone(),
        last_finished_at: last.as_ref().map(|(t, _)| t.timestamp()),
        last: last.map(|(_, s)| s),
    }))
}

static IMPORT_RUNNING: AtomicBool = AtomicBool::new(false);
static IMPORT_ERROR: Mutex<Option<String>> = Mutex::new(None);

fn lock_error() -> std::sync::MutexGuard<'static, Option<String>> {
    IMPORT_ERROR
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

async fn import_now(state: &AppState, sources: Sources) -> Result<(), ApiError> {
    let started_at = chrono::Utc::now();
    let index = cached_index(state, Server::EN).await?;
    let server_data = state
        .try_server_data(Server::EN)
        .ok_or(ApiError::ServiceUnavailable)?;
    let assets_dir = std::path::PathBuf::from(&server_data.assets_dir);
    let mut stories: Vec<_> = index
        .lookup
        .iter()
        .map(|(id, r)| (id.clone(), r.clone()))
        .collect();
    stories.sort_by(|a, b| a.0.cmp(&b.0));
    let gd = server_data.game_data.load_full();
    let mut game = import::GameNames {
        operators: gd
            .operators
            .iter()
            .map(|(id, op)| (id.clone(), op.name.clone()))
            .collect(),
        library: Vec::new(),
    };
    for g in &index.index.groups {
        game.library.push(g.name.clone());
        if let Some(z) = &g.zone {
            game.library
                .extend(z.name_first.iter().chain(&z.chapter_name).cloned());
        }
        for st in &g.stories {
            game.library.push(st.name.clone());
            game.library.extend(st.avg_tag.clone());
        }
    }
    let built =
        tokio::task::spawn_blocking(move || import::build(&sources, &assets_dir, &stories, &game))
            .await
            .map_err(|e| ApiError::Internal(e.into()))??;
    let summary = &built.summary;
    import::store(&state.db, SERVER, started_at, &built).await?;
    edits::db::mark_outdated(&state.db).await?;
    // The glossary rides along: a failed download keeps the stored copy, and
    // the counts are redone over the new lines either way.
    if glossary::refresh(&state.db, &state.http_client)
        .await
        .is_err()
    {
        glossary::recount(&state.db)
            .await
            .map_err(ApiError::Internal)?;
    }
    tracing::info!(
        lines = summary.lines,
        with_ru = summary.with_ru_game,
        "translators import done"
    );
    Ok(())
}

/// Development only: sign in as a local translator account without a game
/// account. Answers 404 unless `TRANSLATORS_DEV_LOGIN=1` is set for the
/// backend process.
#[utoipa::path(
    post,
    path = "/translators/dev-login",
    tag = "translators",
    responses(
        (status = 200, description = "A site token for the development translator.", body = LoginResponse),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn dev_login(State(state): State<AppState>) -> Result<Json<LoginResponse>, ApiError> {
    if !dev_login_enabled() {
        return Err(ApiError::NotFound);
    }
    let server = Server::EN;
    let server_id = server.index() as i16;
    let user = match find_raw_by_uid(&state.db, DEV_UID, server_id).await? {
        Some(u) => u,
        None => create_user(&state.db, DEV_UID, server_id).await?,
    };
    db::add_member(&state.db, user.id, "development sign-in").await?;
    let token = create_token(
        &state.config.jwt_secret,
        &user.id.to_string(),
        DEV_UID,
        server.as_str(),
        &user.role,
        7,
    )
    .map_err(|e| ApiError::Internal(e.into()))?;
    tracing::warn!("translators development sign-in used");
    Ok(Json(LoginResponse {
        token,
        uid: DEV_UID.to_owned(),
        server: server.as_str().to_owned(),
    }))
}

/// Export to the XUnity dictionaries: `approved` (for the game; the lead
/// or the pack's maintainer) or `review` (every proposal not approved, marked REVIEW; any member),
/// as one `00_translators.txt` or as patched copies of 11_ui / 12_story /
/// 13_names, with plugin.csv, context.csv and the report. Members only.
#[utoipa::path(
    get,
    path = "/translators/export",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(export::ExportQuery),
    responses(
        (status = 200, description = "The export's files and its report.", body = export::ExportResult),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn export(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<export::ExportQuery>,
) -> Result<Json<export::ExportResult>, ApiError> {
    let me = require_member(&state, &auth).await?;
    let mut result = run_export(&state, &me, &query).await?;
    match query.only.as_deref() {
        // The list: names and sizes, no contents (a corrected 12_story is ~85 MB);
        // the archive of exactly this export is packed meanwhile.
        Some("") => {
            let files: Vec<(String, Vec<u8>)> = result
                .files
                .iter_mut()
                .map(|f| (f.name.clone(), std::mem::take(&mut f.content).into_bytes()))
                .collect();
            result.report.archive_token = Some(archives::prepare(
                me.id,
                result.report.archive.clone(),
                result.report.at,
                files,
            ));
        }
        Some(name) => result.files.retain(|f| f.name == name),
        None => {}
    }
    Ok(Json(result))
}

/// Build an export for a member allowed to make it.
async fn run_export(
    state: &AppState,
    me: &Member,
    query: &export::ExportQuery,
) -> Result<export::ExportResult, ApiError> {
    let review = match query.mode.as_str() {
        "review" => true,
        "approved" => false,
        _ => return Err(ApiError::BadRequest("mode: approved or review".into())),
    };
    let patch = match query.format.as_str() {
        "patch" => true,
        "file" => false,
        _ => return Err(ApiError::BadRequest("format: file or patch".into())),
    };
    if !review && !me.exports_for_game() {
        return Err(ApiError::Forbidden);
    }
    let sources = Sources::from_env().map_err(ApiError::BadRequest)?;
    let rows = export::rows(&state.db, review).await?;
    let context = export::context_rows(&state.db, review).await?;
    let only_here = export::only_here_rows(&state.db).await?;
    let keys: Vec<String> = rows.iter().map(|r| r.edit_key.clone()).collect();
    let places = export::places(&state.db, &keys).await?;
    // Each story's script file, as the bundles name it.
    let index = cached_index(state, Server::EN).await?;
    let story_files: std::collections::HashMap<String, String> = places
        .iter()
        .map(|p| p.story_id.as_str())
        .chain(context.iter().map(|c| c.story_id.as_str()))
        .filter_map(|id| {
            index
                .lookup
                .get(id)
                .map(|r| (id.to_owned(), r.story_txt.clone()))
        })
        .collect();
    let by = match me.nickname.clone() {
        Some(n) => n,
        None => {
            sqlx::query_scalar::<_, String>("SELECT uid FROM users WHERE id = $1")
                .bind(me.id)
                .fetch_one(&state.db)
                .await?
        }
    };
    let at = chrono::Utc::now().timestamp();
    tokio::task::spawn_blocking(move || {
        export::build(
            &rows,
            &context,
            &only_here,
            &places,
            &story_files,
            review,
            patch,
            &sources.ru_text_dir,
            &by,
            at,
        )
    })
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .map_err(|e| ApiError::Internal(e.into()))
}

/// The whole export as one ZIP: the changed dictionaries (or
/// 00_translators.txt), story_lines.tsv, plugin_strings.tsv and
/// only_here.tsv when there are rows, report.txt. Same rights as the export.
#[utoipa::path(
    get,
    path = "/translators/export/archive",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(export::ExportQuery),
    responses(
        (status = 200, description = "The archive.", content_type = "application/zip", body = Vec<u8>),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn export_archive(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Query(query): Query<export::ExportQuery>,
) -> Result<axum::response::Response, ApiError> {
    let me = require_member(&state, &auth).await?;
    if let Some(token) = query.token.as_deref() {
        let (name, bytes) = archives::take(me.id, token)
            .await
            .ok_or(ApiError::NotFound)?;
        return Ok(zip_response(&name, bytes));
    }
    let result = run_export(&state, &me, &query).await?;
    let name = result.report.archive.clone();
    let at = result.report.at;
    let files: Vec<(String, Vec<u8>)> = result
        .files
        .into_iter()
        .map(|f| (f.name, f.content.into_bytes()))
        .collect();
    let bytes = tokio::task::spawn_blocking(move || export::zip::zip(&files, at))
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .map_err(|e| ApiError::Internal(e.into()))?;
    Ok(zip_response(&name, bytes))
}

fn zip_response(name: &str, bytes: Vec<u8>) -> axum::response::Response {
    use axum::http::header;
    use axum::response::IntoResponse;
    (
        [
            (header::CONTENT_TYPE, "application/zip".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        bytes,
    )
        .into_response()
}

/// Archives packed in the background for a prepared export, kept a while in
/// memory (a few, 30 minutes): packing a corrected dictionary takes longer
/// than the site's server waits on a silent request.
mod archives {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex, PoisonError};
    use std::time::{Duration, Instant};

    use tokio::sync::watch;
    use uuid::Uuid;

    type Packed = Option<Result<Vec<u8>, String>>;

    struct Entry {
        user: Uuid,
        name: String,
        at: Instant,
        rx: watch::Receiver<Packed>,
    }

    const KEEP: Duration = Duration::from_secs(30 * 60);
    const MAX: usize = 4;
    static ARCHIVES: LazyLock<Mutex<HashMap<String, Entry>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    /// Start packing; returns the token to download it with.
    pub fn prepare(user: Uuid, name: String, at: i64, files: Vec<(String, Vec<u8>)>) -> String {
        let token = Uuid::new_v4().simple().to_string();
        let (tx, rx) = watch::channel(None);
        tokio::task::spawn_blocking(move || {
            let packed = super::export::zip::zip(&files, at).map_err(|e| e.to_string());
            let _ = tx.send(Some(packed));
        });
        let mut map = ARCHIVES.lock().unwrap_or_else(PoisonError::into_inner);
        map.retain(|_, e| e.at.elapsed() < KEEP);
        while map.len() >= MAX {
            let Some(oldest) = map.iter().min_by_key(|(_, e)| e.at).map(|(k, _)| k.clone()) else {
                break;
            };
            map.remove(&oldest);
        }
        map.insert(
            token.clone(),
            Entry {
                user,
                name,
                at: Instant::now(),
                rx,
            },
        );
        token
    }

    /// The archive of a token of this user, once packed.
    pub async fn take(user: Uuid, token: &str) -> Option<(String, Vec<u8>)> {
        let (name, mut rx) = {
            let map = ARCHIVES.lock().unwrap_or_else(PoisonError::into_inner);
            let e = map
                .get(token)
                .filter(|e| e.user == user && e.at.elapsed() < KEEP)?;
            (e.name.clone(), e.rx.clone())
        };
        let packed = rx.wait_for(Option::is_some).await.ok()?.clone()?;
        packed.ok().map(|b| (name, b))
    }
}

/// Mark what went out in a pack version: every approved entry for the game
/// not yet out with the same text. The pack's maintainer only.
#[utoipa::path(
    post,
    path = "/translators/releases",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = export::ReleaseInput,
    responses(
        (status = 200, description = "The release with what it marked.", body = export::Release),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn mark_release(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<export::ReleaseInput>,
) -> Result<Json<export::Release>, ApiError> {
    let me = require_member(&state, &auth).await?;
    if !me.releases() {
        return Err(ApiError::Forbidden);
    }
    let version = input.version.trim();
    if version.is_empty() || version.chars().count() > 40 {
        return Err(ApiError::BadRequest("version: 1 to 40 characters".into()));
    }
    let note = input
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    let id = export::mark_release(&state.db, version, note, me.id).await?;
    export::releases(&state.db)
        .await?
        .into_iter()
        .find(|r| r.id == id)
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// Every pack version marked, newest first. Members only.
#[utoipa::path(
    get,
    path = "/translators/releases",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The releases with their counts.", body = Vec<export::Release>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn list_releases(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<export::Release>>, ApiError> {
    require_member(&state, &auth).await?;
    Ok(Json(export::releases(&state.db).await?))
}
