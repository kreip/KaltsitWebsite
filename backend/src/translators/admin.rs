//! The admin panel (admins only): requests for access, the members with
//! their roles, and the statistics. "Request access" itself is for any
//! signed-in user of the site who is not a member yet.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

use super::SERVER;
use super::edits::dto::EditPerson;
use super::routes::{Member, require_member};
use crate::app::error::ApiError;
use crate::app::extractors::auth::MaybeAuthUser;
use crate::app::state::AppState;

/// The roles, least first.
pub const ROLES: &[&str] = &["user", "translator", "admin"];
/// The longest note of a request, in characters.
const MAX_NOTE: usize = 500;

const NAME: &str = "COALESCE(NULLIF(u.nickname, ''), u.uid)";

/// Whether the user's request for access waits.
pub async fn request_open(pool: &PgPool, user: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM translators.access_requests WHERE user_id = $1 AND status = 'open')",
    )
    .bind(user)
    .fetch_one(pool)
    .await
}

async fn require_admin(state: &AppState, auth: &MaybeAuthUser) -> Result<Member, ApiError> {
    let me = require_member(state, auth).await?;
    if !me.administers() {
        return Err(ApiError::Forbidden);
    }
    Ok(me)
}

fn role_of(input: &str) -> Result<&'static str, ApiError> {
    ROLES
        .iter()
        .copied()
        .find(|r| *r == input)
        .ok_or_else(|| ApiError::BadRequest("role: user, translator or admin".into()))
}

fn person(id: Uuid, name: String) -> EditPerson {
    EditPerson {
        id: id.to_string(),
        name,
    }
}

/// "Request access" with an optional note.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessRequestInput {
    #[serde(default)]
    pub note: Option<String>,
}

/// Ask to join the section: a signed-in user who is not a member. Asking
/// again replaces the note (and reopens a declined request).
#[utoipa::path(
    post,
    path = "/translators/access-request",
    tag = "translators",
    security(("bearer_auth" = [])),
    request_body = AccessRequestInput,
    responses(
        (status = 204, description = "The request waits for an admin."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn request_access(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Json(input): Json<AccessRequestInput>,
) -> Result<StatusCode, ApiError> {
    let id = auth
        .0
        .as_ref()
        .and_then(|u| u.user_id.parse::<Uuid>().ok())
        .ok_or(ApiError::Unauthorized)?;
    if super::db::member_role(&state.db, id).await?.is_some() {
        return Err(ApiError::BadRequest("already a member".into()));
    }
    let note = input
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if note.is_some_and(|n| n.chars().count() > MAX_NOTE) {
        return Err(ApiError::BadRequest(format!(
            "the note is longer than {MAX_NOTE} characters"
        )));
    }
    sqlx::query(
        "INSERT INTO translators.access_requests (user_id, note) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET note = EXCLUDED.note, created_at = NOW(),
             status = 'open', decided_by = NULL, decided_at = NULL",
    )
    .bind(id)
    .bind(note)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A request for access.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccessRequest {
    pub user: EditPerson,
    /// The game uid they signed in with.
    pub uid: String,
    pub note: Option<String>,
    /// Unix seconds.
    #[ts(type = "number")]
    pub created_at: i64,
    /// `open`, `accepted` or `declined`.
    pub status: String,
    pub decided_by: Option<EditPerson>,
}

/// The requests: the open ones first, then the latest decided (admins only).
#[utoipa::path(
    get,
    path = "/translators/admin/requests",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The requests, open first, newest first.", body = Vec<AccessRequest>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn list_requests(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<AccessRequest>>, ApiError> {
    require_admin(&state, &auth).await?;
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        Uuid,
        String,
        String,
        Option<String>,
        DateTime<Utc>,
        String,
        Option<Uuid>,
        Option<String>,
    )> = sqlx::query_as(&format!(
        "SELECT r.user_id, {NAME}, u.uid, r.note, r.created_at, r.status, r.decided_by,
                COALESCE(NULLIF(d.nickname, ''), d.uid)
         FROM translators.access_requests r
         JOIN users u ON u.id = r.user_id
         LEFT JOIN users d ON d.id = r.decided_by
         ORDER BY (r.status <> 'open'), COALESCE(r.decided_at, r.created_at) DESC
         LIMIT 100"
    ))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(id, name, uid, note, at, status, by, by_name)| AccessRequest {
                    user: person(id, name),
                    uid,
                    note,
                    created_at: at.timestamp(),
                    status,
                    decided_by: by.map(|b| person(b, by_name.unwrap_or_default())),
                },
            )
            .collect(),
    ))
}

/// An admin's answer to a request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestDecision {
    pub accept: bool,
    /// The new member's role (`user` unless given).
    #[serde(default)]
    pub role: Option<String>,
}

/// Accept a request (the user becomes a member with the role) or decline it (admins only).
#[utoipa::path(
    post,
    path = "/translators/admin/requests/{user_id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("user_id" = String, Path, description = "The user who asked.")),
    request_body = RequestDecision,
    responses(
        (status = 204, description = "Decided."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn decide_request(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(user_id): Path<Uuid>,
    Json(input): Json<RequestDecision>,
) -> Result<StatusCode, ApiError> {
    let me = require_admin(&state, &auth).await?;
    let role = role_of(input.role.as_deref().unwrap_or("user"))?;
    let mut tx = state.db.begin().await?;
    let found = sqlx::query(
        "UPDATE translators.access_requests SET status = $2, decided_by = $3, decided_at = NOW()
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(if input.accept { "accepted" } else { "declined" })
    .bind(me.id)
    .execute(&mut *tx)
    .await?;
    if found.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    if input.accept {
        sqlx::query(
            "INSERT INTO translators.members (user_id, note, roles) VALUES ($1, 'request', ARRAY[$2])
             ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(role)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A member of the section.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MemberRow {
    pub user: EditPerson,
    pub uid: String,
    /// `user`, `translator` or `admin`.
    pub role: String,
    /// Unix seconds.
    #[ts(type = "number")]
    pub added_at: i64,
}

/// Every member, admins first, then by name (admins only).
#[utoipa::path(
    get,
    path = "/translators/admin/members",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The members.", body = Vec<MemberRow>),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn list_members(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<Vec<MemberRow>>, ApiError> {
    require_admin(&state, &auth).await?;
    let rows: Vec<(Uuid, String, String, String, DateTime<Utc>)> = sqlx::query_as(&format!(
        "SELECT m.user_id, {NAME}, u.uid, m.roles[1], m.added_at
         FROM translators.members m JOIN users u ON u.id = m.user_id
         ORDER BY array_position(ARRAY['admin', 'translator', 'user'], m.roles[1]), lower({NAME})"
    ))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, name, uid, role, at)| MemberRow {
                user: person(id, name),
                uid,
                role,
                added_at: at.timestamp(),
            })
            .collect(),
    ))
}

/// A member's new role.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleInput {
    pub role: String,
}

/// Change a member's role (admins only; never one's own, so the section keeps an admin).
#[utoipa::path(
    put,
    path = "/translators/admin/members/{user_id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("user_id" = String, Path, description = "The member.")),
    request_body = RoleInput,
    responses(
        (status = 204, description = "The role is set."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn set_role(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(user_id): Path<Uuid>,
    Json(input): Json<RoleInput>,
) -> Result<StatusCode, ApiError> {
    let me = require_admin(&state, &auth).await?;
    let role = role_of(&input.role)?;
    if user_id == me.id {
        return Err(ApiError::BadRequest("not one's own role".into()));
    }
    let done = sqlx::query("UPDATE translators.members SET roles = ARRAY[$2] WHERE user_id = $1")
        .bind(user_id)
        .bind(role)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Remove a member from the section (admins only; never oneself). Their
/// proposals, votes and remarks stay.
#[utoipa::path(
    delete,
    path = "/translators/admin/members/{user_id}",
    tag = "translators",
    security(("bearer_auth" = [])),
    params(("user_id" = String, Path, description = "The member.")),
    responses(
        (status = 204, description = "Removed."),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 404, response = crate::app::openapi::responses::NotFound),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn remove_member(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let me = require_admin(&state, &auth).await?;
    if user_id == me.id {
        return Err(ApiError::BadRequest("not oneself".into()));
    }
    let done = sqlx::query("DELETE FROM translators.members WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Proposals made, approved and rejected.
#[derive(Debug, Default, Serialize, TS, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditCounts {
    pub proposed: i32,
    pub approved: i32,
    pub rejected: i32,
    /// Lines approved with any variant (the game's text, a translator's, the machine's, a proposal).
    pub lines_approved: i32,
}

/// One person's share.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonStats {
    pub person: EditPerson,
    /// The role, when a member.
    pub role: Option<String>,
    /// Their proposals: the last 7 days and in all.
    pub week: EditCounts,
    pub total: EditCounts,
    /// Approvals and rejections they made, in all.
    pub decisions: i32,
}

/// One section's progress: its texts, those with Russian, approved, waiting.
#[derive(Debug, Serialize, TS, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SectionStats {
    /// `story`, `operators`, `enemies`, `stages`, `items`, `archives`, `other`.
    pub section: String,
    pub total: i32,
    pub ru: i32,
    pub approved: i32,
    pub waiting: i32,
}

/// The admin panel's statistics.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdminStats {
    pub week: EditCounts,
    pub total: EditCounts,
    pub people: Vec<PersonStats>,
    pub sections: Vec<SectionStats>,
}

/// `approved_at` of a proposal: approved for its entry, or for its one place.
const APPROVED_AT: &str = "COALESCE(p.approved_at, (SELECT a.approved_at FROM translators.approvals a
     WHERE md5(a.edit_key) = md5(p.edit_key) AND a.edit_key = p.edit_key AND a.proposal_id = p.id))";

#[derive(sqlx::FromRow)]
struct PersonRow {
    id: Uuid,
    name: String,
    role: Option<String>,
    w_proposed: i32,
    w_approved: i32,
    w_rejected: i32,
    proposed: i32,
    approved: i32,
    rejected: i32,
    decisions: i32,
}

/// Proposals and decisions this week and in all, by person, and each section's progress (admins only).
#[utoipa::path(
    get,
    path = "/translators/admin/stats",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The statistics.", body = AdminStats),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn stats(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<AdminStats>, ApiError> {
    require_admin(&state, &auth).await?;
    let pool = &state.db;
    let people: Vec<PersonRow> = sqlx::query_as(&format!(
        "WITH p AS (
             SELECT p.author_id, p.created_at, p.rejected_at, {APPROVED_AT} AS approved_at
             FROM translators.proposals p WHERE p.deleted_at IS NULL),
         mine AS (
             SELECT author_id AS id,
                 COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '7 days')::int AS w_proposed,
                 COUNT(*) FILTER (WHERE approved_at > NOW() - INTERVAL '7 days')::int AS w_approved,
                 COUNT(*) FILTER (WHERE rejected_at > NOW() - INTERVAL '7 days')::int AS w_rejected,
                 COUNT(*)::int AS proposed,
                 COUNT(approved_at)::int AS approved,
                 COUNT(rejected_at)::int AS rejected
             FROM p GROUP BY author_id),
         decided AS (
             SELECT id, COUNT(*)::int AS decisions FROM (
                 SELECT user_id AS id FROM translators.approval_log WHERE action = 'approve'
                 UNION ALL
                 SELECT rejected_by FROM translators.proposals WHERE rejected_by IS NOT NULL AND deleted_at IS NULL
             ) d GROUP BY id),
         who AS (SELECT user_id AS id FROM translators.members UNION SELECT id FROM mine UNION SELECT id FROM decided)
         SELECT w.id, {NAME} AS name, m.roles[1] AS role,
             COALESCE(x.w_proposed, 0) AS w_proposed, COALESCE(x.w_approved, 0) AS w_approved,
             COALESCE(x.w_rejected, 0) AS w_rejected, COALESCE(x.proposed, 0) AS proposed,
             COALESCE(x.approved, 0) AS approved, COALESCE(x.rejected, 0) AS rejected,
             COALESCE(d.decisions, 0) AS decisions
         FROM who w JOIN users u ON u.id = w.id
         LEFT JOIN translators.members m ON m.user_id = w.id
         LEFT JOIN mine x ON x.id = w.id
         LEFT JOIN decided d ON d.id = w.id
         ORDER BY COALESCE(x.proposed, 0) + COALESCE(d.decisions, 0) DESC, lower({NAME})"
    ))
    .fetch_all(pool)
    .await?;
    let lines: (i32, i32) = sqlx::query_as(
        "SELECT COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '7 days')::int, COUNT(*)::int
         FROM translators.approval_log WHERE action = 'approve'",
    )
    .fetch_one(pool)
    .await?;
    let mut week = EditCounts {
        lines_approved: lines.0,
        ..EditCounts::default()
    };
    let mut total = EditCounts {
        lines_approved: lines.1,
        ..EditCounts::default()
    };
    let people = people
        .into_iter()
        .map(|r| {
            week.proposed += r.w_proposed;
            week.approved += r.w_approved;
            week.rejected += r.w_rejected;
            total.proposed += r.proposed;
            total.approved += r.approved;
            total.rejected += r.rejected;
            PersonStats {
                person: person(r.id, r.name),
                role: r.role,
                week: EditCounts {
                    proposed: r.w_proposed,
                    approved: r.w_approved,
                    rejected: r.w_rejected,
                    lines_approved: 0,
                },
                total: EditCounts {
                    proposed: r.proposed,
                    approved: r.approved,
                    rejected: r.rejected,
                    lines_approved: 0,
                },
                decisions: r.decisions,
            }
        })
        .collect();
    Ok(Json(AdminStats {
        week,
        total,
        people,
        sections: section_stats(pool).await?,
    }))
}

/// Each section's texts: the story by its lines, the game's sections by
/// their texts (a text in two sections counts in both).
pub async fn section_stats(pool: &PgPool) -> Result<Vec<SectionStats>, sqlx::Error> {
    let story: SectionStats = sqlx::query_as(
        "SELECT 'story' AS section, COUNT(*)::int AS total,
             COUNT(*) FILTER (WHERE s.ru_game IS NOT NULL)::int AS ru,
             COUNT(*) FILTER (WHERE a.edit_key IS NOT NULL)::int AS approved,
             COUNT(*) FILTER (WHERE a.edit_key IS NULL AND EXISTS (
                 SELECT 1 FROM translators.proposals p WHERE p.edit_key = s.edit_key
                 AND p.deleted_at IS NULL AND p.rejected_at IS NULL AND p.outdated_at IS NULL))::int AS waiting
         FROM translators.story_lines s
         LEFT JOIN translators.approvals a ON a.edit_key = s.edit_key AND a.outdated_at IS NULL
         WHERE s.server = $1 AND s.edit_key ~ '[[:alnum:]]'",
    )
    .bind(SERVER)
    .fetch_one(pool)
    .await?;
    let mut out = vec![story];
    out.extend(
        sqlx::query_as::<_, SectionStats>(
            "SELECT x.section, COUNT(*)::int AS total,
                 COUNT(*) FILTER (WHERE t.ru IS NOT NULL)::int AS ru,
                 COUNT(*) FILTER (WHERE a.edit_key IS NOT NULL)::int AS approved,
                 COUNT(*) FILTER (WHERE a.edit_key IS NULL AND EXISTS (
                     SELECT 1 FROM translators.proposals p WHERE p.edit_key = t.edit_key
                     AND p.deleted_at IS NULL AND p.rejected_at IS NULL AND p.outdated_at IS NULL))::int AS waiting
             FROM translators.texts t
             CROSS JOIN LATERAL unnest(t.sections) AS x(section)
             LEFT JOIN translators.approvals a ON a.edit_key = t.edit_key AND a.outdated_at IS NULL
             WHERE t.server = $1 AND x.section <> 'story'
             GROUP BY x.section",
        )
        .bind(SERVER)
        .fetch_all(pool)
        .await?,
    );
    Ok(out)
}

/// Start the import from the admin panel (what `translators/import.sh` does
/// with the service key): in the background, re-runnable. Admins only.
#[utoipa::path(
    post,
    path = "/translators/admin/import",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The import's state right after the request.", body = super::dto::ImportStatus),
        (status = 400, response = crate::app::openapi::responses::BadRequest),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn admin_run_import(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<super::dto::ImportStatus>, ApiError> {
    require_admin(&state, &auth).await?;
    super::routes::start_import(&state)?;
    super::routes::import_status_body(&state).await.map(Json)
}

/// The import's state for the admin panel: running (its step and start), the last error, the last summary.
#[utoipa::path(
    get,
    path = "/translators/admin/import",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The import's state.", body = super::dto::ImportStatus),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn admin_import_status(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<super::dto::ImportStatus>, ApiError> {
    require_admin(&state, &auth).await?;
    super::routes::import_status_body(&state).await.map(Json)
}

/// Load the site's Russian around the section (`site-ru.json`) into the
/// site's translations again: keys with no Russian yet, and the section's
/// menu item. Admins only.
#[utoipa::path(
    post,
    path = "/translators/admin/site-texts",
    tag = "translators",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "How many keys the file has, were written, and are unknown to the catalog.", body = super::site_texts::SiteTextsLoaded),
        (status = 401, response = crate::app::openapi::responses::Unauthorized),
        (status = 403, response = crate::app::openapi::responses::Forbidden),
        (status = 500, response = crate::app::openapi::responses::InternalError)
    )
)]
pub async fn refresh_site_texts(
    State(state): State<AppState>,
    auth: MaybeAuthUser,
) -> Result<Json<super::site_texts::SiteTextsLoaded>, ApiError> {
    require_admin(&state, &auth).await?;
    let out = super::site_texts::load(&state.db).await?;
    state.cache.invalidate_by_prefix("i18n:").await;
    Ok(Json(out))
}
