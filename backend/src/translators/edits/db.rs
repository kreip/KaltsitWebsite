//! Queries of line editing (schema `translators`, migration t003).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::dto::{
    Approval, EditComment, EditPerson, EditPlace, EditRow, GameHere, HistoryEvent, KeyEdits,
    KeyPlace, OutdatedEdit, PlaceAgreement, Proposal, ProposalVersion, StoryProgress, StoryRead,
};
use super::status;
use crate::translators::SERVER;
use crate::translators::texts::gamedata::STORY;

/// How a member's name shows: the game nickname, else the uid.
const NAME: &str = "COALESCE(NULLIF(u.nickname, ''), u.uid)";

fn person(id: Uuid, name: String) -> EditPerson {
    EditPerson {
        id: id.to_string(),
        name,
    }
}

/// The line at a place: its edit key, English and the game's Russian.
pub struct PlaceLine {
    pub edit_key: String,
    pub ru_game: Option<String>,
    pub sova: Option<String>,
    pub ai: Option<String>,
}

pub async fn place_line(pool: &PgPool, p: &EditPlace) -> Result<Option<PlaceLine>, sqlx::Error> {
    // A text outside the story: its entry in the list of every text (no Sova, no AI there).
    if p.is_text() {
        let Some(key) = p.edit_key.as_deref() else {
            return Ok(None);
        };
        // A long entry is named by its md5 (`EditPlace::text_by_md5`), else by its English.
        let sql = if EditPlace::md5_of(key).is_some() {
            "SELECT edit_key, ru FROM translators.texts WHERE server = $1 AND md5(edit_key) = $2"
        } else {
            "SELECT edit_key, ru FROM translators.texts WHERE server = $1 AND edit_key = $2"
        };
        let row: Option<(String, Option<String>)> = sqlx::query_as(sql)
            .bind(SERVER)
            .bind(EditPlace::md5_of(key).unwrap_or(key))
            .fetch_optional(pool)
            .await?;
        return Ok(row.map(|(edit_key, ru_game)| PlaceLine {
            edit_key,
            ru_game,
            sova: None,
            ai: None,
        }));
    }
    #[allow(clippy::type_complexity)]
    let row: Option<(String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT edit_key, ru_game, sova, ai FROM translators.story_lines
         WHERE server = $1 AND story_id = $2 AND line = $3 AND part = $4",
    )
    .bind(SERVER)
    .bind(&p.story_id)
    .bind(p.line)
    .bind(p.part)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(edit_key, ru_game, sova, ai)| PlaceLine {
        edit_key,
        ru_game,
        sova,
        ai,
    }))
}

/// The game's Russian for a key (the same at every place).
pub async fn key_ru_game(pool: &PgPool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<Option<String>> = sqlx::query_scalar(
        "SELECT ru FROM (
             (SELECT ru_game AS ru, 0 AS o FROM translators.story_lines
              WHERE server = $1 AND edit_key = $2 LIMIT 1)
             UNION ALL
             (SELECT ru, 1 FROM translators.texts WHERE server = $1 AND edit_key = $2)
         ) x ORDER BY o LIMIT 1",
    )
    .bind(SERVER)
    .bind(key)
    .fetch_optional(pool)
    .await?;
    Ok(row.flatten())
}

/// How many places hold each key: story lines and fields of the game data
/// (the list of every text counts both; the story's count when it is more).
pub async fn place_counts(
    pool: &PgPool,
    keys: &[String],
) -> Result<HashMap<String, i32>, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT edit_key, MAX(n) FROM (
             SELECT edit_key, COUNT(*) AS n FROM translators.story_lines
             WHERE server = $1 AND edit_key = ANY($2) GROUP BY edit_key
             UNION ALL
             SELECT edit_key, places::bigint FROM translators.texts
             WHERE server = $1 AND edit_key = ANY($2)
         ) x GROUP BY edit_key",
    )
    .bind(SERVER)
    .bind(keys)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(k, n)| (k, i32::try_from(n).unwrap_or(i32::MAX)))
        .collect())
}

pub async fn places(pool: &PgPool, key: &str, limit: i64) -> Result<Vec<KeyPlace>, sqlx::Error> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, i32, i16, i32, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT s.story_id, s.line, s.part, s.ord,
                COALESCE(b.ru_game, b.en), COALESCE(a.ru_game, a.en)
         FROM translators.story_lines s
         LEFT JOIN translators.story_lines b
             ON b.server = s.server AND b.story_id = s.story_id AND b.ord = s.ord - 1
         LEFT JOIN translators.story_lines a
             ON a.server = s.server AND a.story_id = s.story_id AND a.ord = s.ord + 1
         WHERE s.server = $1 AND s.edit_key = $2 ORDER BY s.story_id, s.ord LIMIT $3",
    )
    .bind(SERVER)
    .bind(key)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    let mut out: Vec<KeyPlace> = rows
        .into_iter()
        .map(|(story_id, line, part, ord, before, after)| KeyPlace {
            section: STORY.to_owned(),
            story_id,
            line,
            part,
            ord,
            before,
            after,
            object_id: None,
            object_en: None,
            field: None,
            idx: None,
        })
        .collect();
    // Then the fields of the game data holding it.
    let left = limit - i64::try_from(out.len()).unwrap_or(limit);
    if left > 0 {
        let game: Vec<(String, String, String, String, i16)> = sqlx::query_as(
            "SELECT section, object_id, object_en, field, idx FROM translators.text_places
             WHERE server = $1 AND edit_key = $2 ORDER BY section, object_id, ord LIMIT $3",
        )
        .bind(SERVER)
        .bind(key)
        .bind(left)
        .fetch_all(pool)
        .await?;
        out.extend(
            game.into_iter()
                .map(|(section, object_id, object_en, field, idx)| KeyPlace {
                    section,
                    story_id: String::new(),
                    line: 0,
                    part: 0,
                    ord: 0,
                    before: None,
                    after: None,
                    object_id: Some(object_id),
                    object_en: Some(object_en),
                    field: Some(field),
                    idx: Some(idx),
                }),
        );
    }
    Ok(out)
}

#[derive(sqlx::FromRow)]
struct ProposalRow {
    id: i64,
    edit_key: String,
    text: String,
    source: String,
    author_id: Uuid,
    author_name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    story_id: String,
    line: i32,
    part: i16,
    outdated_at: Option<DateTime<Utc>>,
    context: bool,
    approved_by: Option<Uuid>,
    approved_at: Option<DateTime<Utc>>,
    approver_name: Option<String>,
    versions: i64,
    previous_text: Option<String>,
    game_section: Option<String>,
    game_object: Option<String>,
    game_object_en: Option<String>,
    game_field: Option<String>,
    game_idx: Option<i16>,
    rejected_by: Option<Uuid>,
    rejected_at: Option<DateTime<Utc>>,
    reject_reason: Option<String>,
    rejecter_name: Option<String>,
}

const PROPOSAL_COLUMNS: &str = "p.id, p.edit_key, p.text, p.source, p.author_id, \
     COALESCE(NULLIF(u.nickname, ''), u.uid) AS author_name, p.created_at, p.updated_at, \
     p.story_id, p.line, p.part, p.outdated_at, p.context, p.approved_by, p.approved_at, \
     (SELECT COALESCE(NULLIF(x.nickname, ''), x.uid) FROM users x WHERE x.id = p.approved_by) AS approver_name, \
     (SELECT COUNT(*) FROM translators.proposal_versions v WHERE v.proposal_id = p.id) AS versions, \
     (SELECT v.text FROM translators.proposal_versions v WHERE v.proposal_id = p.id \
       ORDER BY v.id DESC OFFSET 1 LIMIT 1) AS previous_text, \
     p.game_section, p.game_object, p.game_object_en, p.game_field, p.game_idx, \
     p.rejected_by, p.rejected_at, p.reject_reason, \
     (SELECT COALESCE(NULLIF(x.nickname, ''), x.uid) FROM users x WHERE x.id = p.rejected_by) AS rejecter_name";

impl ProposalRow {
    fn into_proposal(self, agreements: Vec<EditPerson>) -> Proposal {
        // A rewrite moves `updated_at`; the first save sets both at once.
        let edited = self.versions > 1;
        Proposal {
            id: self.id,
            edit_key: self.edit_key,
            text: self.text,
            source: self.source,
            author: person(self.author_id, self.author_name),
            created_at: self.created_at.timestamp(),
            edited_at: edited.then(|| self.updated_at.timestamp()),
            versions: i32::try_from(self.versions).unwrap_or(i32::MAX),
            previous_text: self.previous_text,
            agreements,
            place: EditPlace {
                story_id: self.story_id,
                line: self.line,
                part: self.part,
                edit_key: None,
            },
            context: self.context,
            here: match (self.game_section, self.game_object, self.game_field) {
                (Some(section), Some(object_id), Some(field)) => Some(GameHere {
                    section,
                    object_id,
                    object_en: self.game_object_en,
                    field,
                    idx: self.game_idx.unwrap_or(0),
                }),
                _ => None,
            },
            outdated: self.outdated_at.is_some(),
            approved_by: self
                .approved_by
                .map(|id| person(id, self.approver_name.unwrap_or_default())),
            approved_at: self.approved_at.map(|t| t.timestamp()),
            released_in: None,
            rejected_by: self
                .rejected_by
                .map(|id| person(id, self.rejecter_name.unwrap_or_default())),
            rejected_at: self.rejected_at.map(|t| t.timestamp()),
            reject_reason: self.reject_reason,
        }
    }
}

/// Who agrees with what, for the keys: `(key, proposal or None) -> people`
/// for the game's Russian and the proposals.
type Votes = HashMap<(String, Option<i64>), Vec<EditPerson>>;

/// Votes for Sova's and the AI's variants: `(key, variant, place) -> people`.
type PlaceVotes = HashMap<(String, String, EditPlaceKey), Vec<EditPerson>>;
type EditPlaceKey = (String, i32, i16);

#[derive(sqlx::FromRow)]
struct VoteRow {
    edit_key: String,
    variant: String,
    proposal_id: Option<i64>,
    story_id: Option<String>,
    line: Option<i32>,
    part: Option<i16>,
    user_id: Uuid,
    name: String,
}

async fn all_votes(pool: &PgPool, keys: &[String]) -> Result<(Votes, PlaceVotes), sqlx::Error> {
    let rows: Vec<VoteRow> = sqlx::query_as(&format!(
        "SELECT a.edit_key, a.variant, a.proposal_id, a.story_id, a.line, a.part, a.user_id, {NAME} AS name
         FROM translators.agreements a JOIN users u ON u.id = a.user_id
         WHERE a.edit_key = ANY($1) ORDER BY a.created_at"
    ))
    .bind(keys)
    .fetch_all(pool)
    .await?;
    let mut votes: Votes = HashMap::new();
    let mut places: PlaceVotes = HashMap::new();
    for r in rows {
        let who = person(r.user_id, r.name);
        match (r.variant.as_str(), r.story_id, r.line, r.part) {
            ("sova" | "ai", Some(story), Some(line), Some(part)) => places
                .entry((r.edit_key, r.variant, (story, line, part)))
                .or_default()
                .push(who),
            _ => votes
                .entry((r.edit_key, r.proposal_id))
                .or_default()
                .push(who),
        }
    }
    Ok((votes, places))
}

async fn votes(pool: &PgPool, keys: &[String]) -> Result<Votes, sqlx::Error> {
    Ok(all_votes(pool, keys).await?.0)
}

fn entry<'m>(out: &'m mut HashMap<String, KeyEdits>, key: &str) -> &'m mut KeyEdits {
    out.entry(key.to_owned()).or_insert_with(|| KeyEdits {
        edit_key: key.to_owned(),
        approval: None,
        game_agreements: Vec::new(),
        place_agreements: Vec::new(),
        proposals: Vec::new(),
        comments: Vec::new(),
    })
}

/// Everything proposed, approved and said for the keys (one entry per key
/// that has anything).
pub async fn key_edits(pool: &PgPool, keys: &[String]) -> Result<Vec<KeyEdits>, sqlx::Error> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let (mut votes, place_votes) = all_votes(pool, keys).await?;
    let proposals: Vec<ProposalRow> = sqlx::query_as(&format!(
        "SELECT {PROPOSAL_COLUMNS} FROM translators.proposals p JOIN users u ON u.id = p.author_id
         WHERE p.deleted_at IS NULL AND p.rejected_at IS NULL AND p.edit_key = ANY($1) ORDER BY p.created_at"
    ))
    .bind(keys)
    .fetch_all(pool)
    .await?;
    #[allow(clippy::type_complexity)]
    let approvals: Vec<(
        String,
        String,
        Option<i64>,
        String,
        Uuid,
        String,
        DateTime<Utc>,
    )> = sqlx::query_as(&format!(
        "SELECT a.edit_key, a.variant, a.proposal_id, a.text, a.approved_by, {NAME}, a.approved_at
             FROM translators.approvals a JOIN users u ON u.id = a.approved_by
             WHERE a.edit_key = ANY($1)"
    ))
    .bind(keys)
    .fetch_all(pool)
    .await?;
    let comments = comments(pool, keys).await?;

    let mut out: HashMap<String, KeyEdits> = HashMap::new();
    for ((key, proposal), people) in &votes {
        if proposal.is_none() {
            entry(&mut out, key).game_agreements.clone_from(people);
        }
    }
    for p in proposals {
        let agreed = votes
            .remove(&(p.edit_key.clone(), Some(p.id)))
            .unwrap_or_default();
        let key = p.edit_key.clone();
        entry(&mut out, &key)
            .proposals
            .push(p.into_proposal(agreed));
    }
    for (key, variant, proposal_id, text, by, name, at) in approvals {
        entry(&mut out, &key).approval = Some(Approval {
            released_in: None,
            variant,
            proposal_id,
            text,
            by: person(by, name),
            at: at.timestamp(),
        });
    }
    for (key, c) in comments {
        entry(&mut out, &key).comments.push(c);
    }
    for ((key, variant, (story_id, line, part)), people) in place_votes {
        entry(&mut out, &key).place_agreements.push(PlaceAgreement {
            variant,
            place: EditPlace {
                story_id,
                line,
                part,
                edit_key: None,
            },
            people,
        });
    }
    // What went out in a pack, with the very text approved.
    #[allow(clippy::type_complexity)]
    let released: Vec<(
        String,
        String,
        Option<String>,
        Option<i32>,
        Option<i16>,
        String,
    )> = sqlx::query_as(
        "SELECT DISTINCT ON (i.edit_key, i.text, i.story_id, i.line, i.part)
                i.edit_key, i.text, i.story_id, i.line, i.part, r.version
         FROM translators.release_items i JOIN translators.releases r ON r.id = i.release_id
         WHERE i.edit_key = ANY($1)
         ORDER BY i.edit_key, i.text, i.story_id, i.line, i.part, r.id DESC",
    )
    .bind(keys)
    .fetch_all(pool)
    .await?;
    for (key, text, story, line, part, version) in released {
        let Some(e) = out.get_mut(&key) else { continue };
        match (story, line, part) {
            (Some(story), Some(line), Some(part)) => {
                for p in &mut e.proposals {
                    if p.context
                        && p.text == text
                        && p.place.story_id == story
                        && p.place.line == line
                        && p.place.part == part
                    {
                        p.released_in = Some(version.clone());
                    }
                }
            }
            _ => {
                if let Some(a) = &mut e.approval
                    && a.text == text
                {
                    a.released_in = Some(version);
                }
            }
        }
    }
    let mut out: Vec<KeyEdits> = out.into_values().collect();
    for e in &mut out {
        // Most agreed first; ties by age (the vector is oldest first, the sort is stable).
        e.proposals
            .sort_by_key(|p| std::cmp::Reverse(p.agreements.len()));
    }
    Ok(out)
}

#[derive(sqlx::FromRow)]
struct CommentRow {
    id: i64,
    edit_key: String,
    text: String,
    author_id: Uuid,
    author_name: String,
    created_at: DateTime<Utc>,
    resolved_by: Option<Uuid>,
    resolver_name: Option<String>,
    resolved_at: Option<DateTime<Utc>>,
    story_id: String,
    line: i32,
    part: i16,
    quote: Option<String>,
    kind: String,
    variant: Option<String>,
    proposal_id: Option<i64>,
    variant_text: Option<String>,
}

async fn comments(
    pool: &PgPool,
    keys: &[String],
) -> Result<Vec<(String, EditComment)>, sqlx::Error> {
    let rows: Vec<CommentRow> = sqlx::query_as(&format!(
        "SELECT c.id, c.edit_key, c.text, c.author_id, {NAME} AS author_name, c.created_at,
                c.resolved_by, COALESCE(NULLIF(r.nickname, ''), r.uid) AS resolver_name, c.resolved_at,
                c.story_id, c.line, c.part, c.quote, c.kind, c.variant, c.proposal_id, c.variant_text
         FROM translators.comments c JOIN users u ON u.id = c.author_id
         LEFT JOIN users r ON r.id = c.resolved_by
         WHERE c.deleted_at IS NULL AND c.edit_key = ANY($1) ORDER BY c.created_at"
    ))
    .bind(keys)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|c| {
            (
                c.edit_key,
                EditComment {
                    id: c.id,
                    text: c.text,
                    author: person(c.author_id, c.author_name),
                    created_at: c.created_at.timestamp(),
                    resolved_by: c
                        .resolved_by
                        .map(|i| person(i, c.resolver_name.unwrap_or_default())),
                    resolved_at: c.resolved_at.map(|t| t.timestamp()),
                    place: EditPlace {
                        story_id: c.story_id,
                        line: c.line,
                        part: c.part,
                        edit_key: None,
                    },
                    quote: c.quote,
                    kind: c.kind,
                    variant: c.variant,
                    proposal_id: c.proposal_id,
                    variant_text: c.variant_text,
                },
            )
        })
        .collect())
}

/// Approvals and proposals left in a story at places whose English changed.
pub async fn outdated_in_story(
    pool: &PgPool,
    story_id: &str,
) -> Result<Vec<OutdatedEdit>, sqlx::Error> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, i32, i16, String, String, Uuid, String, DateTime<Utc>)> =
        sqlx::query_as(&format!(
            "SELECT 'proposal', p.line, p.part, p.edit_key, p.text, p.author_id, {NAME}, p.updated_at
             FROM translators.proposals p JOIN users u ON u.id = p.author_id
             WHERE p.story_id = $1 AND p.deleted_at IS NULL AND p.rejected_at IS NULL AND p.outdated_at IS NOT NULL
             UNION ALL
             SELECT 'approval', a.line, a.part, a.edit_key, a.text, a.approved_by, {NAME}, a.approved_at
             FROM translators.approvals a JOIN users u ON u.id = a.approved_by
             WHERE a.story_id = $1 AND a.outdated_at IS NOT NULL"
        ))
        .bind(story_id)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(kind, line, part, edit_key, text, id, name, at)| OutdatedEdit {
                kind,
                line,
                part,
                edit_key,
                text,
                author: person(id, name),
                at: at.timestamp(),
            },
        )
        .collect())
}

/// The caller's live proposal for a key in a scope (every place, or this
/// place only — a story line, or with `here` a field of a game object), if
/// any: `(id, text, source)`.
async fn my_proposal(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
    author: Uuid,
    context: Option<&EditPlace>,
    here: Option<&GameHere>,
) -> Result<Option<(i64, String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, text, source FROM translators.proposals
         WHERE md5(edit_key) = md5($1) AND edit_key = $1 AND author_id = $2 AND deleted_at IS NULL
           AND context = $3
           AND (NOT $3 OR (story_id = $4 AND line = $5 AND part = $6
                AND game_section IS NOT DISTINCT FROM $7 AND game_object IS NOT DISTINCT FROM $8
                AND game_field IS NOT DISTINCT FROM $9 AND COALESCE(game_idx, 0) = $10))
         FOR UPDATE",
    )
    .bind(key)
    .bind(author)
    .bind(context.is_some())
    .bind(context.map(|p| p.story_id.as_str()))
    .bind(context.map(|p| p.line))
    .bind(context.map(|p| p.part))
    .bind(here.map(|h| h.section.as_str()))
    .bind(here.map(|h| h.object_id.as_str()))
    .bind(here.map(|h| h.field.as_str()))
    .bind(here.map_or(0, |h| h.idx))
    .fetch_optional(&mut **tx)
    .await
}

async fn add_version(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
    text: &str,
    source: &str,
    author: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO translators.proposal_versions (proposal_id, text, source, author_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(text)
    .bind(source)
    .bind(author)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Whether the game data shows the entry at a place.
pub async fn game_place_exists(
    pool: &PgPool,
    key: &str,
    here: &GameHere,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM translators.text_places
                        WHERE server = $1 AND edit_key = $2 AND section = $3 AND object_id = $4
                          AND field = $5 AND idx = $6)",
    )
    .bind(SERVER)
    .bind(key)
    .bind(&here.section)
    .bind(&here.object_id)
    .bind(&here.field)
    .bind(here.idx)
    .fetch_one(pool)
    .await
}

/// Write the caller's proposal for the key (for every place, or for this
/// place only when `context` — outside the story with `here`, the place of
/// the game data), or rewrite the one they have in that scope (a new
/// version; the place moves to where it was rewritten). Returns its id.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_proposal(
    pool: &PgPool,
    key: &str,
    place: &EditPlace,
    text: &str,
    source: &str,
    context: bool,
    here: Option<&GameHere>,
    author: Uuid,
) -> Result<i64, sqlx::Error> {
    let here = here.filter(|_| context);
    let mut tx = pool.begin().await?;
    let mine = my_proposal(&mut tx, key, author, context.then_some(place), here).await?;
    let id = if let Some((id, old_text, old_source)) = mine {
        // Writing it again makes a rejected proposal a proposal again.
        sqlx::query(
            "UPDATE translators.proposals SET rejected_by = NULL, rejected_at = NULL, reject_reason = NULL
             WHERE id = $1 AND rejected_at IS NOT NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if old_text != text || old_source != source {
            sqlx::query(
                "UPDATE translators.proposals SET text = $2, source = $3, updated_at = NOW(),
                        story_id = $4, line = $5, part = $6
                 WHERE id = $1",
            )
            .bind(id)
            .bind(text)
            .bind(source)
            .bind(&place.story_id)
            .bind(place.line)
            .bind(place.part)
            .execute(&mut *tx)
            .await?;
            add_version(&mut tx, id, text, source, author).await?;
        }
        id
    } else {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO translators.proposals (edit_key, text, source, author_id, story_id, line, part, context,
                                                game_section, game_object, game_object_en, game_field, game_idx)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id",
        )
        .bind(key)
        .bind(text)
        .bind(source)
        .bind(author)
        .bind(&place.story_id)
        .bind(place.line)
        .bind(place.part)
        .bind(context)
        .bind(here.map(|h| h.section.as_str()))
        .bind(here.map(|h| h.object_id.as_str()))
        .bind(here.and_then(|h| h.object_en.as_deref()))
        .bind(here.map(|h| h.field.as_str()))
        .bind(here.map(|h| h.idx))
        .fetch_one(&mut *tx)
        .await?;
        add_version(&mut tx, id, text, source, author).await?;
        id
    };
    tx.commit().await?;
    Ok(id)
}

/// A live proposal: its key, author, text, source and whether it is for one place only.
pub async fn proposal_owner(
    pool: &PgPool,
    id: i64,
) -> Result<Option<(String, Uuid, String, String, bool)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT edit_key, author_id, text, source, context FROM translators.proposals WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Rewrite a proposal (the caller checked it is theirs).
pub async fn rewrite_proposal(
    pool: &PgPool,
    id: i64,
    text: &str,
    source: &str,
    author: Uuid,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE translators.proposals SET text = $2, updated_at = NOW(),
             rejected_by = NULL, rejected_at = NULL, reject_reason = NULL WHERE id = $1",
    )
        .bind(id)
        .bind(text)
        .execute(&mut *tx)
        .await?;
    add_version(&mut tx, id, text, source, author).await?;
    tx.commit().await
}

/// Soft-delete a proposal and drop the votes for it.
pub async fn delete_proposal(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE translators.proposals SET deleted_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM translators.agreements WHERE proposal_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

pub async fn versions(pool: &PgPool, id: i64) -> Result<Vec<ProposalVersion>, sqlx::Error> {
    let rows: Vec<(i64, String, String, Uuid, String, DateTime<Utc>)> = sqlx::query_as(&format!(
        "SELECT v.id, v.text, v.source, v.author_id, {NAME}, v.created_at
         FROM translators.proposal_versions v JOIN users u ON u.id = v.author_id
         WHERE v.proposal_id = $1 ORDER BY v.id DESC"
    ))
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(vid, text, source, id, name, at)| ProposalVersion {
            id: vid,
            text,
            source,
            author: person(id, name),
            created_at: at.timestamp(),
        })
        .collect())
}

/// The game's Russian before the last import that changed it, by entry: `(text, when)`.
pub async fn ru_before(
    pool: &PgPool,
    keys: &[String],
) -> Result<HashMap<String, (String, DateTime<Utc>)>, sqlx::Error> {
    if keys.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT DISTINCT ON (edit_key) edit_key, text, changed_at FROM translators.ru_history
         WHERE server = $1 AND edit_key = ANY($2) ORDER BY edit_key, id DESC",
    )
    .bind(SERVER)
    .bind(keys)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(k, t, at)| (k, (t, at))).collect())
}

/// The history of the entry's translation as seen at a place, newest first:
/// the pack's changes, every text of every proposal (those for another place
/// only are left out; deleted ones are kept) and the lead's decisions.
pub async fn history(
    pool: &PgPool,
    key: &str,
    place: &EditPlace,
    ru_game: Option<&str>,
) -> Result<Vec<HistoryEvent>, sqlx::Error> {
    let mut events: Vec<HistoryEvent> = Vec::new();
    // The pack: each row is the Russian before a change; what followed it is the next row's, then today's.
    let game: Vec<(String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT text, changed_at FROM translators.ru_history WHERE server = $1 AND edit_key = $2 ORDER BY id",
    )
    .bind(SERVER)
    .bind(key)
    .fetch_all(pool)
    .await?;
    for (i, (before, at)) in game.iter().enumerate() {
        events.push(HistoryEvent {
            kind: "game".to_owned(),
            at: at.timestamp(),
            by: None,
            text: game
                .get(i + 1)
                .map(|(t, _)| t.clone())
                .or_else(|| ru_game.map(str::to_owned)),
            before: Some(before.clone()),
            proposal_id: None,
            source: None,
            context: false,
            deleted: false,
        });
    }
    #[allow(clippy::type_complexity)]
    let versions: Vec<(i64, String, String, Uuid, String, DateTime<Utc>, bool, bool)> =
        sqlx::query_as(&format!(
            "SELECT p.id, v.text, v.source, v.author_id, {NAME}, v.created_at, p.context,
                    (p.deleted_at IS NOT NULL)
             FROM translators.proposal_versions v
             JOIN translators.proposals p ON p.id = v.proposal_id
             JOIN users u ON u.id = v.author_id
             WHERE p.edit_key = $1
               AND (NOT p.context OR (p.story_id = $2 AND p.line = $3 AND p.part = $4))
             ORDER BY v.id"
        ))
        .bind(key)
        .bind(&place.story_id)
        .bind(place.line)
        .bind(place.part)
        .fetch_all(pool)
        .await?;
    let mut last: HashMap<i64, String> = HashMap::new();
    for (id, text, source, author, name, at, context, deleted) in versions {
        let before = last
            .insert(id, text.clone())
            .or_else(|| ru_game.map(str::to_owned));
        events.push(HistoryEvent {
            kind: "proposal".to_owned(),
            at: at.timestamp(),
            by: Some(person(author, name)),
            text: Some(text),
            before,
            proposal_id: Some(id),
            source: Some(source),
            context,
            deleted,
        });
    }
    #[allow(clippy::type_complexity)]
    let log: Vec<(
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        Uuid,
        String,
        DateTime<Utc>,
    )> = sqlx::query_as(&format!(
        "SELECT l.action, l.proposal_id, l.text, l.variant, l.user_id, {NAME}, l.created_at
             FROM translators.approval_log l JOIN users u ON u.id = l.user_id
             WHERE l.edit_key = $1 ORDER BY l.id"
    ))
    .bind(key)
    .fetch_all(pool)
    .await?;
    // A decision about a one-place proposal shows at that place only.
    let elsewhere: std::collections::HashSet<i64> = sqlx::query_scalar(
        "SELECT id FROM translators.proposals
         WHERE edit_key = $1 AND context AND NOT (story_id = $2 AND line = $3 AND part = $4)",
    )
    .bind(key)
    .bind(&place.story_id)
    .bind(place.line)
    .bind(place.part)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    for (action, proposal, text, variant, user, name, at) in log {
        if proposal.is_some_and(|id| elsewhere.contains(&id)) {
            continue;
        }
        events.push(HistoryEvent {
            kind: match action.as_str() {
                "withdraw" | "reject" | "reopen" => action.clone(),
                _ => "approve".to_owned(),
            },
            at: at.timestamp(),
            by: Some(person(user, name)),
            text,
            before: None,
            proposal_id: proposal,
            source: variant,
            context: false,
            deleted: false,
        });
    }
    // Newest first; steps of one moment keep their order (a proposal before its approval).
    events.reverse();
    events.sort_by_key(|e| std::cmp::Reverse(e.at));
    Ok(events)
}

/// One member's vote for a variant of an entry: `game`, a `proposal`, or
/// `sova` / `ai` at a place (with that variant's text as it is now).
pub struct Vote<'a> {
    pub key: &'a str,
    pub variant: &'a str,
    pub proposal: Option<i64>,
    pub place: Option<&'a EditPlace>,
    pub text: Option<&'a str>,
}

pub async fn set_agreement(
    pool: &PgPool,
    v: &Vote<'_>,
    user: Uuid,
    agree: bool,
) -> Result<(), sqlx::Error> {
    let (story, line, part) = match v.place {
        Some(p) => (Some(p.story_id.as_str()), Some(p.line), Some(p.part)),
        None => (None, None, None),
    };
    if agree {
        sqlx::query(
            "INSERT INTO translators.agreements
                 (edit_key, variant, proposal_id, story_id, line, part, variant_text, user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT DO NOTHING",
        )
        .bind(v.key)
        .bind(v.variant)
        .bind(v.proposal)
        .bind(story)
        .bind(line)
        .bind(part)
        .bind(v.text)
        .bind(user)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "DELETE FROM translators.agreements
             WHERE edit_key = $1 AND variant = $2 AND proposal_id IS NOT DISTINCT FROM $3
               AND story_id IS NOT DISTINCT FROM $4 AND line IS NOT DISTINCT FROM $5
               AND part IS NOT DISTINCT FROM $6 AND user_id = $7",
        )
        .bind(v.key)
        .bind(v.variant)
        .bind(v.proposal)
        .bind(story)
        .bind(line)
        .bind(part)
        .bind(user)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Approve a variant for an entry (replacing an earlier approval), and log it.
#[allow(clippy::too_many_arguments)]
pub async fn approve_key(
    pool: &PgPool,
    key: &str,
    variant: &str,
    proposal: Option<i64>,
    text: &str,
    place: &EditPlace,
    by: Uuid,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO translators.approvals (edit_key, variant, proposal_id, text, approved_by, story_id, line, part)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (md5(edit_key)) DO UPDATE SET variant = EXCLUDED.variant, proposal_id = EXCLUDED.proposal_id,
             text = EXCLUDED.text, approved_by = EXCLUDED.approved_by, approved_at = NOW(),
             story_id = EXCLUDED.story_id, line = EXCLUDED.line, part = EXCLUDED.part, outdated_at = NULL",
    )
    .bind(key)
    .bind(variant)
    .bind(proposal)
    .bind(text)
    .bind(by)
    .bind(&place.story_id)
    .bind(place.line)
    .bind(place.part)
    .execute(&mut *tx)
    .await?;
    log_approval(
        &mut tx,
        key,
        variant,
        proposal,
        Some(text),
        "approve",
        place,
        by,
    )
    .await?;
    tx.commit().await
}

/// Withdraw the approval of an entry, and log it.
pub async fn withdraw_key(
    pool: &PgPool,
    key: &str,
    place: &EditPlace,
    by: Uuid,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let gone: Option<(String, Option<i64>)> = sqlx::query_as(
        "DELETE FROM translators.approvals WHERE edit_key = $1 RETURNING variant, proposal_id",
    )
    .bind(key)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((variant, proposal)) = gone {
        log_approval(
            &mut tx, key, &variant, proposal, None, "withdraw", place, by,
        )
        .await?;
    }
    tx.commit().await
}

/// Approve a proposal for one place (that place only), or withdraw it, and log it.
pub async fn approve_context(
    pool: &PgPool,
    key: &str,
    proposal: i64,
    place: &EditPlace,
    by: Uuid,
    approve: bool,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE translators.proposals SET approved_by = $2,
             approved_at = CASE WHEN $2::uuid IS NULL THEN NULL ELSE NOW() END
         WHERE id = $1",
    )
    .bind(proposal)
    .bind(approve.then_some(by))
    .execute(&mut *tx)
    .await?;
    let action = if approve { "approve" } else { "withdraw" };
    log_approval(
        &mut tx,
        key,
        "proposal",
        Some(proposal),
        None,
        action,
        place,
        by,
    )
    .await?;
    tx.commit().await
}

#[allow(clippy::too_many_arguments)]
async fn log_approval(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
    variant: &str,
    proposal: Option<i64>,
    text: Option<&str>,
    action: &str,
    place: &EditPlace,
    by: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO translators.approval_log (edit_key, proposal_id, text, action, user_id, variant, story_id, line, part)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(key)
    .bind(proposal)
    .bind(text)
    .bind(action)
    .bind(by)
    .bind(variant)
    .bind(&place.story_id)
    .bind(place.line)
    .bind(place.part)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Who has read a story, first readers first.
pub async fn story_reads(pool: &PgPool, story_id: &str) -> Result<Vec<StoryRead>, sqlx::Error> {
    let rows: Vec<(Uuid, String, DateTime<Utc>)> = sqlx::query_as(&format!(
        "SELECT r.user_id, {NAME}, r.read_at FROM translators.story_reads r
         JOIN users u ON u.id = r.user_id WHERE r.story_id = $1 ORDER BY r.read_at"
    ))
    .bind(story_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, at)| StoryRead {
            by: person(id, name),
            at: at.timestamp(),
        })
        .collect())
}

pub async fn set_read(
    pool: &PgPool,
    story_id: &str,
    user: Uuid,
    read: bool,
) -> Result<(), sqlx::Error> {
    let sql = if read {
        "INSERT INTO translators.story_reads (story_id, user_id) VALUES ($1, $2)
         ON CONFLICT (story_id, user_id) DO UPDATE SET read_at = NOW()"
    } else {
        "DELETE FROM translators.story_reads WHERE story_id = $1 AND user_id = $2"
    };
    sqlx::query(sql)
        .bind(story_id)
        .bind(user)
        .execute(pool)
        .await?;
    Ok(())
}

/// Every story's progress over its lines that can be edited (a letter or a
/// digit): approved (the entry's approval is current, the game's Russian
/// checked included, or a one-place proposal approved here), waiting (a live
/// proposal for every place or for this one, not approved, not outdated) and
/// outdated (a decision left here for a previous English, not approved).
pub async fn progress(pool: &PgPool) -> Result<Vec<StoryProgress>, sqlx::Error> {
    sqlx::query_as(
        "WITH keyed AS (
             SELECT DISTINCT edit_key FROM translators.proposals
             WHERE deleted_at IS NULL AND rejected_at IS NULL AND NOT context AND outdated_at IS NULL),
         placed AS (
             SELECT DISTINCT story_id, line, part FROM translators.proposals
             WHERE deleted_at IS NULL AND rejected_at IS NULL AND context AND outdated_at IS NULL),
         placed_ok AS (
             SELECT DISTINCT story_id, line, part FROM translators.proposals
             WHERE deleted_at IS NULL AND context AND approved_at IS NOT NULL),
         stale AS (
             SELECT story_id, line, part FROM translators.proposals
             WHERE deleted_at IS NULL AND rejected_at IS NULL AND outdated_at IS NOT NULL
             UNION
             SELECT story_id, line, part FROM translators.approvals WHERE outdated_at IS NOT NULL)
         SELECT s.story_id,
                COUNT(*)::int AS total,
                COUNT(*) FILTER (WHERE a.edit_key IS NOT NULL OR q.story_id IS NOT NULL)::int AS approved,
                COUNT(*) FILTER (WHERE a.edit_key IS NULL AND q.story_id IS NULL AND o.story_id IS NULL
                                 AND (k.edit_key IS NOT NULL OR p.story_id IS NOT NULL))::int AS waiting,
                COUNT(*) FILTER (WHERE a.edit_key IS NULL AND q.story_id IS NULL AND o.story_id IS NOT NULL)::int AS outdated
         FROM translators.story_lines s
         LEFT JOIN translators.approvals a ON a.edit_key = s.edit_key AND a.outdated_at IS NULL
         LEFT JOIN keyed k ON k.edit_key = s.edit_key
         LEFT JOIN placed p ON p.story_id = s.story_id AND p.line = s.line AND p.part = s.part
         LEFT JOIN placed_ok q ON q.story_id = s.story_id AND q.line = s.line AND q.part = s.part
         LEFT JOIN (SELECT DISTINCT story_id, line, part FROM stale) o
             ON o.story_id = s.story_id AND o.line = s.line AND o.part = s.part
         WHERE s.server = $1 AND s.edit_key ~ '[[:alnum:]]'
         GROUP BY s.story_id",
    )
    .bind(SERVER)
    .fetch_all(pool)
    .await
}

/// What a new comment or note is: its kind and, for a note on one variant,
/// which one and its text as it is now.
pub struct NewComment<'a> {
    pub text: &'a str,
    pub quote: Option<&'a str>,
    pub kind: &'a str,
    pub variant: Option<&'a str>,
    pub proposal_id: Option<i64>,
    pub variant_text: Option<&'a str>,
}

pub async fn add_comment(
    pool: &PgPool,
    key: &str,
    place: &EditPlace,
    c: &NewComment<'_>,
    author: Uuid,
) -> Result<i64, sqlx::Error> {
    let text = c.text;
    let quote = c.quote;
    sqlx::query_scalar(
        "INSERT INTO translators.comments
             (edit_key, text, author_id, story_id, line, part, quote, kind, variant, proposal_id, variant_text)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(key)
    .bind(text)
    .bind(author)
    .bind(&place.story_id)
    .bind(place.line)
    .bind(place.part)
    .bind(quote)
    .bind(c.kind)
    .bind(c.variant)
    .bind(c.proposal_id)
    .bind(c.variant_text)
    .fetch_one(pool)
    .await
}

/// A live comment's author and kind.
pub async fn comment_author(pool: &PgPool, id: i64) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT author_id, kind FROM translators.comments WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_comment(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE translators.comments SET deleted_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn resolve_comment(pool: &PgPool, id: i64, by: Option<Uuid>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE translators.comments SET resolved_by = $2, resolved_at = CASE WHEN $2::uuid IS NULL THEN NULL ELSE NOW() END
         WHERE id = $1",
    )
    .bind(id)
    .bind(by)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct EditRowDb {
    #[sqlx(flatten)]
    p: ProposalRow,
    en: Option<String>,
    ru_game: Option<String>,
    ord: Option<i32>,
    places: i64,
    approved: bool,
    approved_proposal: Option<i64>,
}

/// "My edits": the caller's live proposals (or everyone's), newest first.
pub async fn edit_rows(pool: &PgPool, author: Option<Uuid>) -> Result<Vec<EditRow>, sqlx::Error> {
    let rows: Vec<EditRowDb> = sqlx::query_as(&format!(
        "SELECT {PROPOSAL_COLUMNS},
                k.en, k.ru_game, here.ord,
                GREATEST((SELECT COUNT(*) FROM translators.story_lines s
                           WHERE s.server = $1 AND s.edit_key = p.edit_key),
                         (SELECT t.places FROM translators.texts t
                           WHERE t.server = $1 AND t.edit_key = p.edit_key), 0) AS places,
                (a.edit_key IS NOT NULL) AS approved, a.proposal_id AS approved_proposal
         FROM translators.proposals p
         JOIN users u ON u.id = p.author_id
         LEFT JOIN LATERAL (
             SELECT x.en, x.ru_game FROM (
                 (SELECT s.en, s.ru_game, 0 AS o FROM translators.story_lines s
                  WHERE s.server = $1 AND s.edit_key = p.edit_key
                  ORDER BY (s.story_id = p.story_id AND s.line = p.line AND s.part = p.part) DESC
                  LIMIT 1)
                 UNION ALL
                 (SELECT t.en, t.ru, 1 FROM translators.texts t
                  WHERE t.server = $1 AND t.edit_key = p.edit_key)
             ) x ORDER BY x.o LIMIT 1
         ) k ON TRUE
         LEFT JOIN translators.story_lines here
             ON here.server = $1 AND here.story_id = p.story_id AND here.line = p.line
            AND here.part = p.part
         LEFT JOIN translators.approvals a ON a.edit_key = p.edit_key
         WHERE p.deleted_at IS NULL AND ($2::uuid IS NULL OR p.author_id = $2)
         ORDER BY p.updated_at DESC
         LIMIT 2000"
    ))
    .bind(SERVER)
    .bind(author)
    .fetch_all(pool)
    .await?;
    let keys: Vec<String> = rows.iter().map(|r| r.p.edit_key.clone()).collect();
    let mut votes = votes(pool, &keys).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let id = r.p.id;
            let outdated = r.p.outdated_at.is_some();
            let rejected = r.p.rejected_at.is_some();
            // A one-place proposal is approved on its own; it never stands for the entry.
            let approved = if r.p.context {
                r.p.approved_at.is_some().then_some(Some(id))
            } else {
                r.approved.then_some(r.approved_proposal)
            };
            let en = r.en.unwrap_or_else(|| r.p.edit_key.clone());
            let agreed = votes
                .remove(&(r.p.edit_key.clone(), Some(id)))
                .unwrap_or_default();
            EditRow {
                proposal: r.p.into_proposal(agreed),
                en,
                ru_game: r.ru_game,
                ord: r.ord,
                state: if rejected {
                    "rejected"
                } else {
                    status::proposal_state(outdated, approved, id)
                }
                .to_owned(),
                places: i32::try_from(r.places).unwrap_or(i32::MAX),
            }
        })
        .collect())
}

/// After an import: mark what is keyed by an English the story no longer
/// has, and clear the mark where it came back.
pub async fn mark_outdated(pool: &PgPool) -> Result<(), sqlx::Error> {
    for table in ["proposals", "approvals", "comments"] {
        sqlx::query(&format!(
            "UPDATE translators.{table} t SET outdated_at = CASE
                 WHEN EXISTS (SELECT 1 FROM translators.story_lines s
                              WHERE s.server = $1 AND s.edit_key = t.edit_key)
                   OR EXISTS (SELECT 1 FROM translators.texts x
                              WHERE x.server = $1 AND x.edit_key = t.edit_key)
                 THEN NULL ELSE COALESCE(t.outdated_at, NOW()) END"
        ))
        .bind(SERVER)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Every named story sprite.
pub async fn char_names(pool: &PgPool) -> Result<Vec<super::dto::CharName>, sqlx::Error> {
    sqlx::query_as(
        "SELECT char_id, name_en, name_ru, guessed FROM translators.char_names ORDER BY char_id",
    )
    .fetch_all(pool)
    .await
}

/// Sova's notes to a story's chapter, in order.
pub async fn sova_notes(pool: &PgPool, story_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT text FROM translators.sova_notes WHERE story_id = $1 ORDER BY idx")
        .bind(story_id)
        .fetch_all(pool)
        .await
}

/// The library's names with the game's Russian.
pub async fn names_ru(pool: &PgPool) -> Result<Vec<super::dto::NameRu>, sqlx::Error> {
    sqlx::query_as("SELECT en, ru FROM translators.names_ru ORDER BY en")
        .fetch_all(pool)
        .await
}

/// What a decision on one proposal needs: its entry, text, place and whether
/// it is approved, rejected or outdated.
#[derive(sqlx::FromRow)]
pub struct DecisionTarget {
    pub edit_key: String,
    pub text: String,
    pub story_id: String,
    pub line: i32,
    pub part: i16,
    pub context: bool,
    pub outdated: bool,
    pub rejected: bool,
    /// Approved: for its entry, or (one place) for its place.
    pub approved: bool,
}

pub async fn decision_target(pool: &PgPool, id: i64) -> Result<Option<DecisionTarget>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.edit_key, p.text, p.story_id, p.line, p.part, p.context,
                (p.outdated_at IS NOT NULL) AS outdated, (p.rejected_at IS NOT NULL) AS rejected,
                CASE WHEN p.context THEN p.approved_at IS NOT NULL
                     ELSE EXISTS (SELECT 1 FROM translators.approvals a
                                  WHERE md5(a.edit_key) = md5(p.edit_key) AND a.edit_key = p.edit_key
                                    AND a.proposal_id = p.id) END AS approved
         FROM translators.proposals p WHERE p.id = $1 AND p.deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Reject a proposal (with a reason its author sees) or take the rejection
/// back (`reason` ignored), and log it. The caller withdrew its approval.
pub async fn set_rejected(
    pool: &PgPool,
    id: i64,
    key: &str,
    place: &EditPlace,
    reject: bool,
    reason: Option<&str>,
    by: Uuid,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE translators.proposals SET rejected_by = $2,
             rejected_at = CASE WHEN $2::uuid IS NULL THEN NULL ELSE NOW() END, reject_reason = $3
         WHERE id = $1",
    )
    .bind(id)
    .bind(reject.then_some(by))
    .bind(reason.filter(|_| reject))
    .execute(&mut *tx)
    .await?;
    let action = if reject { "reject" } else { "reopen" };
    log_approval(
        &mut tx,
        key,
        "proposal",
        Some(id),
        reason.filter(|_| reject),
        action,
        place,
        by,
    )
    .await?;
    tx.commit().await
}
