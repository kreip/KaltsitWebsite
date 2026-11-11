//! Storing the list of texts and searching it.

use std::collections::HashMap;

use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use super::dto::{TextAt, TextHit, TextQuery};
use super::{TextPlace, TextRow, fold_row};

/// Replace the server's list inside the import's transaction. The search
/// index is dropped for the load and built again after it: far faster than
/// keeping it up to date row by row.
pub async fn store(
    tx: &mut Transaction<'_, Postgres>,
    server: &str,
    rows: &[TextRow],
    places: &[TextPlace],
) -> Result<(), sqlx::Error> {
    sqlx::query("DROP INDEX IF EXISTS translators.texts_fold_trgm")
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM translators.texts WHERE server = $1")
        .bind(server)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM translators.text_places WHERE server = $1")
        .bind(server)
        .execute(&mut **tx)
        .await?;
    // 10 binds per row, under Postgres's 65,535-parameter limit.
    for chunk in rows.chunks(5_000) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO translators.texts (server, edit_key, en, ru, file, plugin_role, ru_plugin, sections, places, fold) ",
        );
        qb.push_values(chunk, |mut b, r| {
            b.push_bind(server)
                .push_bind(&r.edit_key)
                .push_bind(&r.en)
                .push_bind(&r.ru)
                .push_bind(&r.file)
                .push_bind(&r.plugin_role)
                .push_bind(&r.ru_plugin)
                .push_bind(
                    r.sections
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect::<Vec<String>>(),
                )
                .push_bind(i32::try_from(r.places).unwrap_or(i32::MAX))
                .push_bind(fold_row(&r.en, r.ru.as_deref()));
        });
        // One row per entry (the unique index on the key's hash); the builder never repeats one.
        qb.push(" ON CONFLICT DO NOTHING");
        qb.build().execute(&mut **tx).await?;
    }
    for chunk in places.chunks(8_000) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO translators.text_places (server, edit_key, section, object_id, object_en, field, idx, ord) ",
        );
        qb.push_values(chunk, |mut b, p| {
            b.push_bind(server)
                .push_bind(&p.edit_key)
                .push_bind(p.section)
                .push_bind(&p.object_id)
                .push_bind(&p.object_en)
                .push_bind(&p.field)
                .push_bind(p.idx)
                .push_bind(p.ord);
        });
        qb.build().execute(&mut **tx).await?;
    }
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS texts_fold_trgm ON translators.texts USING gin (fold gin_trgm_ops)",
    )
    .execute(&mut **tx)
    .await?;
    // Fresh statistics: the planner's choices and the list's size (`indexed`).
    sqlx::query("ANALYZE translators.texts")
        .execute(&mut **tx)
        .await?;
    sqlx::query("ANALYZE translators.text_places")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// `%`, `_` and `\` of a query taken literally by LIKE.
fn like(q: &str) -> String {
    let mut out = String::with_capacity(q.len() + 2);
    out.push('%');
    for c in q.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

/// The query as a regular expression matching it as whole words.
fn whole_words(q: &str) -> String {
    let mut out = String::from("\\m");
    for c in q.chars() {
        if !c.is_alphanumeric() && !c.is_whitespace() {
            out.push('\\');
        }
        out.push(c);
    }
    out.push_str("\\M");
    out
}

const APPROVED: &str = "EXISTS (SELECT 1 FROM translators.approvals a WHERE a.edit_key = t.edit_key AND a.outdated_at IS NULL)";
const PROPOSED: &str = "EXISTS (SELECT 1 FROM translators.proposals p WHERE p.edit_key = t.edit_key AND p.deleted_at IS NULL AND p.outdated_at IS NULL)";

/// The WHERE of a search, after `FROM translators.texts t`.
fn conditions<'q>(
    qb: &mut QueryBuilder<'q, Postgres>,
    server: &'q str,
    q: &'q TextQuery,
    folded: &'q str,
) {
    qb.push(" WHERE t.server = ").push_bind(server);
    if !folded.is_empty() {
        if q.whole {
            qb.push(" AND t.fold ~ ").push_bind(whole_words(folded));
        } else {
            qb.push(" AND t.fold LIKE ").push_bind(like(folded));
        }
    }
    if let Some(section) = q.section.as_deref().filter(|s| !s.is_empty()) {
        qb.push(" AND t.sections @> ARRAY[")
            .push_bind(section)
            .push("]::text[]");
    }
    match q.status.as_deref() {
        Some("approved") => {
            qb.push(format!(" AND {APPROVED}"));
        }
        Some("proposed") => {
            qb.push(format!(" AND {PROPOSED} AND NOT {APPROVED}"));
        }
        Some("unchecked") => {
            qb.push(format!(" AND NOT {PROPOSED} AND NOT {APPROVED}"));
        }
        Some("untranslated") => {
            qb.push(" AND t.ru IS NULL");
        }
        Some("plugin") => {
            qb.push(" AND t.plugin_role IS NOT NULL");
        }
        _ => {}
    }
}

#[derive(sqlx::FromRow)]
struct HitRow {
    id: i64,
    edit_key: String,
    en: String,
    ru: Option<String>,
    file: Option<String>,
    plugin_role: Option<String>,
    ru_plugin: Option<String>,
    sections: Vec<String>,
    places: i32,
    status: String,
}

/// How many places of a found text come with it.
const PLACES_SHOWN: usize = 3;

/// One page of the texts matching the query (shortest English first, so a
/// name comes before the sentences that mention it) and how many match.
pub async fn search(
    pool: &PgPool,
    server: &str,
    q: &TextQuery,
) -> Result<(i64, Vec<TextHit>), sqlx::Error> {
    let folded = super::fold(q.q.trim());
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);

    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM translators.texts t");
    conditions(&mut count, server, q, &folded);

    // The page is cut first and the states are read for its rows only, so the scan may run in parallel.
    let mut page: QueryBuilder<Postgres> = QueryBuilder::new(format!(
        "SELECT t.*, CASE WHEN {APPROVED} THEN 'approved' WHEN {PROPOSED} THEN 'proposed' ELSE 'unchecked' END AS status
         FROM (SELECT t.id, t.edit_key, t.en, t.ru, t.file, t.plugin_role, t.ru_plugin, t.sections, t.places
               FROM translators.texts t"
    ));
    conditions(&mut page, server, q, &folded);
    page.push(" ORDER BY length(t.en), t.id LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset)
        .push(") t ORDER BY length(t.en), t.id");
    // Not kept prepared: a kept statement soon gets one plan for every query, and the right
    // plan depends on the words (the index for a rare word, the whole table for "the" or two letters).
    let (total, rows): (i64, Vec<HitRow>) = tokio::try_join!(
        count.build_query_scalar().persistent(false).fetch_one(pool),
        page.build_query_as().persistent(false).fetch_all(pool)
    )?;

    let keys: Vec<String> = rows.iter().map(|r| r.edit_key.clone()).collect();
    let mut at: HashMap<String, Vec<TextAt>> = HashMap::new();
    if !keys.is_empty() {
        #[allow(clippy::type_complexity)]
        let game: Vec<(String, String, String, String, String, i16)> = sqlx::query_as(
            "SELECT edit_key, section, object_id, object_en, field, idx FROM (
                 SELECT p.*, ROW_NUMBER() OVER (PARTITION BY p.edit_key ORDER BY p.section, p.object_id, p.ord) AS n
                 FROM translators.text_places p WHERE p.server = $1 AND p.edit_key = ANY($2)
             ) x WHERE n <= $3 ORDER BY edit_key, n",
        )
        .bind(server)
        .bind(&keys)
        .bind(i64::try_from(PLACES_SHOWN).unwrap_or(3))
        .fetch_all(pool)
        .await?;
        for (key, section, object_id, object_en, field, idx) in game {
            at.entry(key).or_default().push(TextAt {
                section,
                story_id: None,
                line: None,
                part: None,
                ord: None,
                object_id: Some(object_id),
                object_en: Some(object_en),
                field: Some(field),
                idx: Some(idx),
            });
        }
        let story: Vec<(String, String, i32, i16, i32)> = sqlx::query_as(
            "SELECT edit_key, story_id, line, part, ord FROM (
                 SELECT s.edit_key, s.story_id, s.line, s.part, s.ord,
                        ROW_NUMBER() OVER (PARTITION BY s.edit_key ORDER BY s.story_id, s.ord) AS n
                 FROM translators.story_lines s WHERE s.server = $1 AND s.edit_key = ANY($2)
             ) x WHERE n <= $3 ORDER BY edit_key, n",
        )
        .bind(server)
        .bind(&keys)
        .bind(i64::try_from(PLACES_SHOWN).unwrap_or(3))
        .fetch_all(pool)
        .await?;
        for (key, story_id, line, part, ord) in story {
            let list = at.entry(key).or_default();
            if list.len() < PLACES_SHOWN {
                list.push(TextAt {
                    section: super::gamedata::STORY.to_owned(),
                    story_id: Some(story_id),
                    line: Some(line),
                    part: Some(part),
                    ord: Some(ord),
                    object_id: None,
                    object_en: None,
                    field: None,
                    idx: None,
                });
            }
        }
    }
    let hits = rows
        .into_iter()
        .map(|r| TextHit {
            at: at.remove(&r.edit_key).unwrap_or_default(),
            id: r.id,
            edit_key: r.edit_key,
            en: r.en,
            ru: r.ru,
            file: r.file,
            plugin_role: r.plugin_role,
            ru_plugin: r.ru_plugin,
            sections: r.sections,
            places: r.places,
            status: r.status,
        })
        .collect();
    Ok((total, hits))
}

/// How many texts the list holds, exactly.
pub async fn indexed(pool: &PgPool, server: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM translators.texts WHERE server = $1")
        .bind(server)
        .fetch_one(pool)
        .await
}

/// How many texts the list holds, as of the last import's statistics: read
/// with every search, so not counted each time.
pub async fn indexed_about(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let n: Option<f32> = sqlx::query_scalar(
        "SELECT reltuples FROM pg_class WHERE oid = 'translators.texts'::regclass",
    )
    .fetch_optional(pool)
    .await?;
    #[allow(clippy::cast_possible_truncation)]
    Ok(n.map_or(0, |n| n.max(0.0).round() as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_is_taken_literally() {
        assert_eq!(like("50%_a\\b"), "%50\\%\\_a\\\\b%");
        assert_eq!(whole_words("rhodes island"), "\\mrhodes island\\M");
        assert_eq!(whole_words("a.b (c)"), "\\ma\\.b \\(c\\)\\M");
    }
}
