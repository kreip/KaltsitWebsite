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
    // 9 binds per row.
    for chunk in places.chunks(7_000) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO translators.text_places (server, edit_key, section, object_id, object_en, field, idx, note, ord) ",
        );
        qb.push_values(chunk, |mut b, p| {
            b.push_bind(server)
                .push_bind(&p.edit_key)
                .push_bind(p.section)
                .push_bind(&p.object_id)
                .push_bind(&p.object_en)
                .push_bind(&p.field)
                .push_bind(p.idx)
                .push_bind(&p.note)
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

/// What names an object of a section: an operator is one record of the game
/// data (its id); elsewhere the game data keeps several records under one
/// name (a stage and its challenge mode, an enemy and its event twin), and
/// they are one object, by the name.
fn object_key(section: &str) -> &'static str {
    if section == super::gamedata::OPERATORS {
        "p.object_id"
    } else {
        "p.object_en"
    }
}

/// A stage's code from its label and name (`0-1 Collapse`, `Collapse` ->
/// `0-1`): the label itself when the stage has no name.
pub fn stage_code(label: &str, name: Option<&str>) -> Option<String> {
    let code = match name {
        Some(name) => label.strip_suffix(name)?.trim(),
        None => label.trim(),
    };
    (!code.is_empty()).then(|| code.to_owned())
}

#[derive(sqlx::FromRow)]
struct ObjectRow {
    id: String,
    en: String,
    name_en: Option<String>,
    name_ru: Option<String>,
    total: i32,
    approved: i32,
    waiting: i32,
}

/// Every object of a section with its progress over its texts that can be
/// edited (a letter or a digit), each English once: approved (the entry's
/// approval is current) and waiting (a live proposal for every place, not approved).
pub async fn objects(
    pool: &PgPool,
    server: &str,
    section: &str,
) -> Result<Vec<super::dto::GameObject>, sqlx::Error> {
    let key = object_key(section);
    let rows: Vec<ObjectRow> = sqlx::query_as(&format!(
        "WITH keyed AS (
             SELECT DISTINCT edit_key FROM translators.proposals
             WHERE deleted_at IS NULL AND NOT context AND outdated_at IS NULL)
         SELECT {key} AS id,
                MIN(p.object_en) AS en,
                MIN(t.en) AS name_en,
                MIN(t.ru) AS name_ru,
                COUNT(DISTINCT p.edit_key) FILTER (WHERE p.edit_key ~ '[[:alnum:]]')::int AS total,
                COUNT(DISTINCT p.edit_key) FILTER (WHERE p.edit_key ~ '[[:alnum:]]'
                    AND a.edit_key IS NOT NULL)::int AS approved,
                COUNT(DISTINCT p.edit_key) FILTER (WHERE p.edit_key ~ '[[:alnum:]]'
                    AND a.edit_key IS NULL AND k.edit_key IS NOT NULL)::int AS waiting
         FROM translators.text_places p
         LEFT JOIN translators.texts t
             ON p.field = 'name' AND t.server = p.server AND t.edit_key = p.edit_key
         LEFT JOIN translators.approvals a ON a.edit_key = p.edit_key AND a.outdated_at IS NULL
         LEFT JOIN keyed k ON k.edit_key = p.edit_key
         WHERE p.server = $1 AND p.section = $2
         GROUP BY 1 ORDER BY 1"
    ))
    .bind(server)
    .bind(section)
    .fetch_all(pool)
    .await?;
    let stages = section == super::gamedata::STAGES;
    let mut out: Vec<super::dto::GameObject> = rows
        .into_iter()
        .map(|r| {
            // A stage's Russian name stands after its code, as its English one does.
            let ru = match (r.name_ru, stages) {
                (Some(ru), true) => Some(match stage_code(&r.en, r.name_en.as_deref()) {
                    Some(code) => format!("{code} {ru}"),
                    None => ru,
                }),
                (ru, _) => ru,
            };
            super::dto::GameObject {
                id: r.id,
                en: r.en,
                ru,
                total: r.total,
                approved: r.approved,
                waiting: r.waiting,
                also: Vec::new(),
                hint: None,
            }
        })
        .collect();
    if key == "p.object_id" {
        tell_twins(pool, server, section, &mut out).await?;
    }
    Ok(out)
}

/// One text of a record: its field, number, entry and the words shown for it.
type TwinText = (String, i16, String, String);

/// Records of a section under one name (`Bench` three times among the
/// devices): those whose texts are the same become one object (`also` keeps
/// the others' ids), and each of the rest gets a `hint` — its first text the
/// others lack, else its id.
async fn tell_twins(
    pool: &PgPool,
    server: &str,
    section: &str,
    objects: &mut Vec<super::dto::GameObject>,
) -> Result<(), sqlx::Error> {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, o) in objects.iter().enumerate() {
        by_name.entry(o.en.as_str()).or_default().push(i);
    }
    let groups: Vec<Vec<usize>> = by_name.into_values().filter(|g| g.len() > 1).collect();
    if groups.is_empty() {
        return Ok(());
    }
    let ids: Vec<String> = groups
        .iter()
        .flatten()
        .map(|&i| objects[i].id.clone())
        .collect();
    let rows: Vec<(String, String, i16, String, String)> = sqlx::query_as(
        "SELECT p.object_id, p.field, p.idx, p.edit_key, COALESCE(t.ru, t.en)
         FROM translators.text_places p
         JOIN translators.texts t ON t.server = p.server AND t.edit_key = p.edit_key
         WHERE p.server = $1 AND p.section = $2 AND p.object_id = ANY($3)
         ORDER BY p.object_id, p.ord",
    )
    .bind(server)
    .bind(section)
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let mut texts: HashMap<String, Vec<TwinText>> = HashMap::new();
    for (id, field, idx, key, words) in rows {
        texts.entry(id).or_default().push((field, idx, key, words));
    }
    let mut gone: Vec<usize> = Vec::new();
    for group in groups {
        let (kept, merged) = twins(&group, objects, &texts);
        for (i, also, hint) in kept {
            objects[i].also = also;
            objects[i].hint = hint;
        }
        gone.extend(merged);
    }
    gone.sort_unstable();
    for i in gone.into_iter().rev() {
        objects.remove(i);
    }
    Ok(())
}

/// How many characters of a text a hint shows.
const HINT_CHARS: usize = 80;

/// Of the records under one name: which stay (with the ids of the records
/// the same as each, and its hint when several stay) and which are merged away.
#[allow(clippy::type_complexity)]
fn twins(
    group: &[usize],
    objects: &[super::dto::GameObject],
    texts: &HashMap<String, Vec<TwinText>>,
) -> (Vec<(usize, Vec<String>, Option<String>)>, Vec<usize>) {
    let empty = Vec::new();
    let of = |i: usize| texts.get(&objects[i].id).unwrap_or(&empty);
    let mut order: Vec<usize> = group.to_vec();
    order.sort_by(|a, b| objects[*a].id.cmp(&objects[*b].id));
    // Records with the same texts at the same fields are one.
    let mut kept: Vec<(usize, Vec<String>)> = Vec::new();
    let mut merged: Vec<usize> = Vec::new();
    for i in order {
        let same = kept.iter_mut().find(|(k, _)| {
            let (a, b) = (of(*k), of(i));
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(x, y)| (&x.0, x.1, &x.2) == (&y.0, y.1, &y.2))
        });
        match same {
            Some((_, also)) => {
                also.push(objects[i].id.clone());
                merged.push(i);
            }
            None => kept.push((i, Vec::new())),
        }
    }
    let alone = kept.len() == 1;
    let out = kept
        .iter()
        .map(|(i, also)| {
            let hint = (!alone).then(|| {
                // Its first text none of the others has; the name is what they share.
                of(*i)
                    .iter()
                    .filter(|t| t.0 != "name")
                    .find(|t| {
                        kept.iter()
                            .filter(|(k, _)| k != i)
                            .all(|(k, _)| !of(*k).iter().any(|x| x.2 == t.2))
                    })
                    .map_or_else(
                        || objects[*i].id.clone(),
                        |t| {
                            let plain = crate::translators::import::normalize::plain(&t.3);
                            let mut hint: String = plain.chars().take(HINT_CHARS).collect();
                            if plain.chars().count() > HINT_CHARS {
                                hint.push('…');
                            }
                            hint
                        },
                    )
            });
            (*i, also.clone(), hint)
        })
        .collect();
    (out, merged)
}

/// One text of an object: what of the object it is, and its entry.
pub struct ObjectText {
    pub field: String,
    pub idx: i16,
    /// What tells it from the object's other texts at this field (`GamePlace::note`).
    pub note: String,
    pub hit: TextHit,
}

#[derive(sqlx::FromRow)]
struct ObjectTextRow {
    field: String,
    idx: i16,
    note: String,
    #[sqlx(flatten)]
    hit: HitRow,
}

/// An object's texts in the game data's order; a text several records of
/// the object hold at one field (a stage's name and its challenge mode's)
/// once, with the notes of them all. A record merged into the object
/// (`GameObject::also`) is found by its own id too.
pub async fn object_texts(
    pool: &PgPool,
    server: &str,
    section: &str,
    object: &str,
) -> Result<Vec<ObjectText>, sqlx::Error> {
    let key = object_key(section);
    let rows: Vec<ObjectTextRow> = sqlx::query_as(&format!(
        "SELECT x.field, x.idx, x.note, t.id, t.edit_key, t.en, t.ru, t.file, t.plugin_role, t.ru_plugin,
                t.sections, t.places, '' AS status
         FROM (SELECT p.field, p.idx, p.edit_key, MIN(p.ord) AS ord, MIN(p.object_id) AS object_id,
                      COALESCE(string_agg(DISTINCT NULLIF(p.note, ''), ','), '') AS note
               FROM translators.text_places p
               WHERE p.server = $1 AND p.section = $2 AND {key} = $3
               GROUP BY p.field, p.idx, p.edit_key) x
         JOIN translators.texts t ON t.server = $1 AND t.edit_key = x.edit_key
         ORDER BY x.ord, x.object_id, t.id"
    ))
    .bind(server)
    .bind(section)
    .bind(object)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ObjectText {
            field: r.field,
            idx: r.idx,
            note: r.note,
            hit: TextHit {
                at: Vec::new(),
                id: r.hit.id,
                edit_key: r.hit.edit_key,
                en: r.hit.en,
                ru: r.hit.ru,
                file: r.hit.file,
                plugin_role: r.hit.plugin_role,
                ru_plugin: r.hit.ru_plugin,
                sections: r.hit.sections,
                places: r.hit.places,
                status: r.hit.status,
            },
        })
        .collect())
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

    #[test]
    fn a_stage_code_is_its_label_without_the_name() {
        assert_eq!(
            stage_code("0-1 Collapse", Some("Collapse")).as_deref(),
            Some("0-1")
        );
        assert_eq!(stage_code("TR-1", None).as_deref(), Some("TR-1"));
        // A stage named without a code, and a name the label does not end with.
        assert_eq!(stage_code("Collapse", Some("Collapse")), None);
        assert_eq!(stage_code("0-1 Collapse", Some("Ruin")), None);
    }

    fn object(id: &str) -> crate::translators::texts::dto::GameObject {
        crate::translators::texts::dto::GameObject {
            id: id.to_owned(),
            en: "Bench".to_owned(),
            ru: None,
            total: 0,
            approved: 0,
            waiting: 0,
            also: Vec::new(),
            hint: None,
        }
    }

    #[test]
    fn records_under_one_name_merge_when_the_same_and_get_a_hint_when_not() {
        let objects = vec![object("trap_3"), object("trap_1"), object("trap_2")];
        let text = |field: &str, key: &str| (field.to_owned(), 0, key.to_owned(), key.to_owned());
        let texts: HashMap<String, Vec<TwinText>> = HashMap::from([
            (
                "trap_1".to_owned(),
                vec![
                    text("name", "Bench"),
                    text("skill.description", "Rest here."),
                ],
            ),
            (
                "trap_2".to_owned(),
                vec![
                    text("name", "Bench"),
                    text("skill.description", "Makes gold."),
                ],
            ),
            (
                "trap_3".to_owned(),
                vec![
                    text("name", "Bench"),
                    text("skill.description", "Rest here."),
                ],
            ),
        ]);
        let (kept, merged) = twins(&[0, 1, 2], &objects, &texts);
        // trap_3 has trap_1's texts: it is trap_1's card.
        assert_eq!(merged, vec![0]);
        assert_eq!(
            kept,
            vec![
                (1, vec!["trap_3".to_owned()], Some("Rest here.".to_owned())),
                (2, Vec::new(), Some("Makes gold.".to_owned())),
            ]
        );
        // All the same: one card, nothing to tell apart.
        let (kept, merged) = twins(&[0, 1], &objects, &texts);
        assert_eq!(kept, vec![(1, vec!["trap_3".to_owned()], None)]);
        assert_eq!(merged, vec![0]);
    }

    #[test]
    fn an_operator_is_a_record_and_the_rest_go_by_name() {
        assert_eq!(object_key("operators"), "p.object_id");
        assert_eq!(object_key("stages"), "p.object_en");
    }
}
