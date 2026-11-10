//! The section's own migrations and queries (schema `translators`).

use sqlx::PgPool;
use uuid::Uuid;

use super::dto::TranslatorLine;

/// Ordered migrations, tracked in `translators._migrations`, apart from the
/// upstream ledger.
const MIGRATIONS: &[(&str, &str)] = &[
    ("t001_init", include_str!("migrations/t001_init.sql")),
    (
        "t002_glossary",
        include_str!("migrations/t002_glossary.sql"),
    ),
    ("t003_edits", include_str!("migrations/t003_edits.sql")),
    ("t004_context", include_str!("migrations/t004_context.sql")),
    ("t005_names", include_str!("migrations/t005_names.sql")),
    (
        "t006_votes_reads",
        include_str!("migrations/t006_votes_reads.sql"),
    ),
    ("t007_approve", include_str!("migrations/t007_approve.sql")),
    (
        "t008_roles_releases",
        include_str!("migrations/t008_roles_releases.sql"),
    ),
    ("t009_history", include_str!("migrations/t009_history.sql")),
    (
        "t010_settings",
        include_str!("migrations/t010_settings.sql"),
    ),
];

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "CREATE SCHEMA IF NOT EXISTS translators;
         CREATE TABLE IF NOT EXISTS translators._migrations (
             name TEXT PRIMARY KEY,
             applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
         );",
    )
    .execute(pool)
    .await?;
    let applied: std::collections::HashSet<String> =
        sqlx::query_scalar("SELECT name FROM translators._migrations")
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();
    for (name, sql) in MIGRATIONS {
        if applied.contains(*name) {
            continue;
        }
        let mut tx = pool.begin().await?;
        sqlx::raw_sql(sql).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO translators._migrations (name) VALUES ($1)")
            .bind(name)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        tracing::info!(migration = name, "applied translators migration");
    }
    Ok(())
}

/// The member's roles (`translator`, `editor`, `maintainer`, `admin`; several
/// at once) and game nickname, or `None` for a non-member.
pub async fn member_role(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<(Vec<String>, Option<String>)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT m.roles, NULLIF(u.nickname, '') FROM translators.members m
         JOIN users u ON u.id = m.user_id WHERE m.user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// The rule that marks a line's recommended variant: `A` or `B` (`B` when the lead never chose).
pub async fn recommend_rule(pool: &PgPool) -> Result<String, sqlx::Error> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM translators.settings WHERE key = 'recommend_rule'")
            .fetch_optional(pool)
            .await?;
    Ok(value.filter(|v| v == "A").unwrap_or_else(|| "B".to_owned()))
}

/// Set the rule for the whole team (the caller checked it is the lead and the value is `A` or `B`).
pub async fn set_recommend_rule(pool: &PgPool, rule: &str, by: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO translators.settings (key, value, updated_by) VALUES ('recommend_rule', $1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by,
             updated_at = NOW()",
    )
    .bind(rule)
    .bind(by)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_member(pool: &PgPool, user_id: Uuid, note: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO translators.members (user_id, note) VALUES ($1, $2) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(note)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct Row {
    line: i32,
    part: i16,
    kind: String,
    speaker_en: String,
    speaker_ru: Option<String>,
    en: String,
    ru_game: Option<String>,
    ru_game_source: Option<String>,
    ru_game_match: Option<String>,
    plugin_role: Option<String>,
    ru_dict: Option<String>,
    speaker_cn: Option<String>,
    cn: Option<String>,
    sova: Option<String>,
    ai: Option<String>,
    edit_key: String,
}

/// One story's lines in file order. Sova's and the AI's texts only when
/// `hints` (the caller is a signed-in translator); otherwise they never leave
/// the database.
pub async fn story_lines(
    pool: &PgPool,
    server: &str,
    story_id: &str,
    hints: bool,
) -> Result<Vec<TranslatorLine>, sqlx::Error> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT line, part, kind, speaker_en, speaker_ru, en, ru_game, ru_game_source, ru_game_match,
                plugin_role, ru_dict, speaker_cn, cn, edit_key,
                CASE WHEN $3 THEN sova END AS sova,
                CASE WHEN $3 THEN ai END AS ai
         FROM translators.story_lines
         WHERE server = $1 AND story_id = $2
         ORDER BY ord",
    )
    .bind(server)
    .bind(story_id)
    .bind(hints)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| TranslatorLine {
            line: u32::try_from(r.line).unwrap_or(0),
            part: r.part,
            kind: r.kind,
            speaker_en: r.speaker_en,
            speaker_ru: r.speaker_ru,
            en: r.en,
            ru_game: r.ru_game,
            ru_game_before: None,
            ru_game_changed_at: None,
            ru_game_source: r.ru_game_source,
            ru_game_match: r.ru_game_match,
            plugin_role: r.plugin_role,
            ru_dict: r.ru_dict,
            speaker_cn: r.speaker_cn,
            cn: r.cn,
            sova: r.sova,
            ai: r.ai,
            terms: Vec::new(),
            editable: super::edits::status::editable(&r.edit_key),
            // Every story line may get a proposal for its place only.
            context_allowed: true,
            edit_key: r.edit_key,
            places: 1,
            status: String::new(),
            variant_glossary: Vec::new(),
        })
        .collect())
}

/// When the server's lines were last imported, if ever.
pub async fn last_import(
    pool: &PgPool,
    server: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
    sqlx::query_scalar("SELECT MAX(finished_at) FROM translators.imports WHERE server = $1")
        .bind(server)
        .fetch_one(pool)
        .await
}

/// The last finished import's time and summary.
pub async fn last_import_summary(
    pool: &PgPool,
    server: &str,
) -> Result<Option<(chrono::DateTime<chrono::Utc>, serde_json::Value)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT finished_at, summary FROM translators.imports WHERE server = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(server)
    .fetch_optional(pool)
    .await
}
