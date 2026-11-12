//! The team glossary: downloaded from its Google Sheet, kept in the section's
//! tables, matched against the story lines. Read only: the sheet is where the
//! team edits it.

pub mod matcher;
pub mod parse;
pub mod suggest;

use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, QueryBuilder};

use matcher::Finder;
use parse::Term;

/// The team's sheet, as people open it.
pub const SHEET_URL: &str =
    "https://docs.google.com/spreadsheets/d/1GVqVvFctKpuRIdaAY4roMuxVaaNMJ8me2ll3mm47ZpE";

/// Its xlsx export, readable without signing in (`TRANSLATORS_GLOSSARY_URL` overrides).
fn export_url() -> String {
    std::env::var("TRANSLATORS_GLOSSARY_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("{SHEET_URL}/export?format=xlsx"))
}

static REFRESHING: AtomicBool = AtomicBool::new(false);

pub fn refreshing() -> bool {
    REFRESHING.load(Ordering::SeqCst)
}

/// Download, parse and store the glossary, then recount it over the story.
/// On any failure the stored copy stays and the error is recorded.
pub async fn refresh(pool: &PgPool, http: &reqwest::Client) -> anyhow::Result<usize> {
    if REFRESHING.swap(true, Ordering::SeqCst) {
        anyhow::bail!("a glossary refresh is already running");
    }
    let result = refresh_inner(pool, http).await;
    REFRESHING.store(false, Ordering::SeqCst);
    if let Err(e) = &result {
        tracing::warn!(error = %e, "glossary refresh failed; keeping the stored copy");
        let _ = sqlx::query(
            "INSERT INTO translators.glossary_meta (id, attempted_at, last_error) VALUES (1, NOW(), $1)
             ON CONFLICT (id) DO UPDATE SET attempted_at = NOW(), last_error = $1",
        )
        .bind(e.to_string())
        .execute(pool)
        .await;
    }
    result
}

async fn refresh_inner(pool: &PgPool, http: &reqwest::Client) -> anyhow::Result<usize> {
    let res = http
        .get(export_url())
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?
        .error_for_status()?;
    let bytes = res.bytes().await?;
    let sha = hex::encode(Sha256::digest(&bytes));
    let terms = tokio::task::spawn_blocking(move || parse::parse(&bytes)).await??;
    if terms.is_empty() {
        anyhow::bail!("the downloaded glossary has no terms");
    }
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM translators.glossary_terms")
        .execute(&mut *tx)
        .await?;
    for chunk in terms.chunks(1_000) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO translators.glossary_terms (sheet, row_no, en, key, qualifier, ru, ru_plural, note, synonyms, comment, cn, case_sensitive, from_sova) ",
        );
        qb.push_values(chunk, |mut b, t| {
            b.push_bind(&t.sheet)
                .push_bind(i32::try_from(t.row).unwrap_or(0))
                .push_bind(&t.en)
                .push_bind(&t.key)
                .push_bind(&t.qualifier)
                .push_bind(&t.ru)
                .push_bind(&t.ru_plural)
                .push_bind(&t.note)
                .push_bind(&t.synonyms)
                .push_bind(&t.comment)
                .push_bind(&t.cn)
                .push_bind(t.case_sensitive)
                .push_bind(t.from_sova);
        });
        qb.build().execute(&mut *tx).await?;
    }
    sqlx::query(
        "INSERT INTO translators.glossary_meta (id, fetched_at, sha256, attempted_at, last_error) VALUES (1, NOW(), $1, NOW(), NULL)
         ON CONFLICT (id) DO UPDATE SET fetched_at = NOW(), sha256 = $1, attempted_at = NOW(), last_error = NULL",
    )
    .bind(&sha)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    recount(pool).await?;
    tracing::info!(terms = terms.len(), "glossary refreshed");
    Ok(terms.len())
}

#[derive(sqlx::FromRow)]
pub struct StoredTerm {
    pub id: i32,
    pub sheet: String,
    pub row_no: i32,
    pub en: String,
    pub key: String,
    pub qualifier: Option<String>,
    pub ru: String,
    pub ru_plural: Option<String>,
    pub note: Option<String>,
    pub synonyms: Option<String>,
    pub comment: Option<String>,
    pub cn: Option<String>,
    pub case_sensitive: bool,
    pub from_sova: bool,
    pub story_lines: i32,
    pub game_variants: sqlx::types::Json<Vec<GameVariant>>,
    pub checked_lines: i32,
    pub found_lines: i32,
    pub cn_game: Option<String>,
}

impl StoredTerm {
    pub fn as_term(&self) -> Term {
        Term {
            sheet: self.sheet.clone(),
            row: u32::try_from(self.row_no).unwrap_or(0),
            en: self.en.clone(),
            key: self.key.clone(),
            qualifier: self.qualifier.clone(),
            ru: self.ru.clone(),
            ru_plural: self.ru_plural.clone(),
            note: self.note.clone(),
            synonyms: self.synonyms.clone(),
            comment: self.comment.clone(),
            cn: self.cn.clone(),
            case_sensitive: self.case_sensitive,
            from_sova: self.from_sova,
        }
    }
}

pub async fn load(pool: &PgPool) -> Result<Vec<StoredTerm>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM translators.glossary_terms ORDER BY id")
        .fetch_all(pool)
        .await
}

/// `(fetched_at, attempted_at, last_error)` of the stored glossary.
pub async fn meta(
    pool: &PgPool,
) -> Result<
    Option<(
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    )>,
    sqlx::Error,
> {
    sqlx::query_as(
        "SELECT fetched_at, attempted_at, last_error FROM translators.glossary_meta WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
}

/// One way the game translates a term, from the Russian of its lines.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameVariant {
    /// The word as the game writes it most often.
    pub ru: String,
    /// Lines of the term that hold the word (any ending).
    pub lines: i32,
    /// The word is the glossary's form.
    pub glossary: bool,
}

/// What [`recount`] works out for one term.
#[derive(Debug, Default, Clone)]
pub struct TermStats {
    pub story_lines: i32,
    pub checked_lines: i32,
    pub found_lines: i32,
    pub game_variants: Vec<GameVariant>,
    pub cn_game: Option<String>,
}

/// One story line as the stats read it.
pub struct StatLine {
    pub en: String,
    pub ru_game: Option<String>,
    pub speaker_en: String,
    pub speaker_cn: Option<String>,
}

/// The stems of a line's Cyrillic words of three letters or more (`ё` = `е`),
/// each with the word as written.
fn ru_stems(s: &str) -> Vec<(String, String)> {
    s.split(|c: char| !c.is_alphabetic())
        .filter(|w| {
            w.chars().count() >= 3 && w.chars().all(|c| ('\u{0400}'..='\u{04FF}').contains(&c))
        })
        .map(|w| {
            (
                matcher::stem(&w.to_lowercase().replace('ё', "е")),
                w.to_owned(),
            )
        })
        .collect()
}

/// A word counts as a variant of a term when it is in at least two of the
/// term's lines, in at least 5 % of them, and a quarter or more of all the
/// story lines holding it are the term's (so "это", "что" never are).
const VARIANT_MIN_LINES: u32 = 2;
const VARIANT_MIN_SHARE: f64 = 0.05;
const VARIANT_MIN_PRECISION: f64 = 0.25;
const VARIANTS_SHOWN: usize = 5;

/// Per term: its story lines, how many have a Russian and hold the glossary's
/// form, the Russian words the game uses for it, and its Chinese from the
/// speakers' names.
pub fn stats(terms: &[Term], lines: &[StatLine]) -> Vec<TermStats> {
    use std::collections::{HashMap, HashSet};
    let finder = Finder::new(terms);
    let mut by_term: Vec<Vec<usize>> = vec![Vec::new(); terms.len()];
    let mut df: HashMap<String, u32> = HashMap::new();
    for (i, l) in lines.iter().enumerate() {
        let mut seen = HashSet::new();
        for f in finder.find(&l.en) {
            for t in f.terms {
                if seen.insert(t) {
                    by_term[t].push(i);
                }
            }
        }
        if let Some(ru) = &l.ru_game {
            let uniq: HashSet<String> = ru_stems(ru).into_iter().map(|w| w.0).collect();
            for w in uniq {
                *df.entry(w).or_default() += 1;
            }
        }
    }
    // Speakers: English name (lower case) -> Chinese name -> lines.
    let mut speakers: HashMap<String, HashMap<String, u32>> = HashMap::new();
    for l in lines {
        if let Some(cn) = l.speaker_cn.as_deref().filter(|c| !c.trim().is_empty())
            && !l.speaker_en.is_empty()
        {
            *speakers
                .entry(l.speaker_en.to_lowercase())
                .or_default()
                .entry(cn.to_owned())
                .or_default() += 1;
        }
    }
    terms
        .iter()
        .zip(&by_term)
        .map(|(term, idx)| {
            let mut out = TermStats {
                story_lines: i32::try_from(idx.len()).unwrap_or(i32::MAX),
                ..TermStats::default()
            };
            let with_ru: Vec<&str> = idx
                .iter()
                .filter_map(|&i| lines[i].ru_game.as_deref())
                .collect();
            out.checked_lines = i32::try_from(with_ru.len()).unwrap_or(i32::MAX);
            out.found_lines = i32::try_from(
                with_ru
                    .iter()
                    .filter(|r| matcher::term_found(r, term))
                    .count(),
            )
            .unwrap_or(i32::MAX);
            // Stems in the term's lines; a stem that starts with a shorter one of
            // four letters or more is the same word with another ending
            // ("заражённых" -> "заражен"), counted under the shorter one.
            let mut all_stems: Vec<String> = with_ru
                .iter()
                .flat_map(|ru| ru_stems(ru).into_iter().map(|(st, _)| st))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            all_stems.sort_by_key(|st| st.chars().count());
            let mut canonical: HashMap<String, String> = HashMap::new();
            let mut roots: Vec<String> = Vec::new();
            for st in &all_stems {
                let root = roots
                    .iter()
                    .find(|r| r.chars().count() >= 4 && st.starts_with(r.as_str()))
                    .cloned();
                match root {
                    Some(r) => {
                        canonical.insert(st.clone(), r);
                    }
                    None => {
                        roots.push(st.clone());
                        canonical.insert(st.clone(), st.clone());
                    }
                }
            }
            // (lines, spellings) per root, and the story-wide lines of each root.
            let mut counts: HashMap<String, (u32, HashMap<String, u32>)> = HashMap::new();
            for ru in &with_ru {
                let mut seen = HashSet::new();
                for (st, orig) in ru_stems(ru) {
                    let root = canonical.get(&st).cloned().unwrap_or(st);
                    let e = counts.entry(root.clone()).or_default();
                    if seen.insert(root) {
                        e.0 += 1;
                    }
                    *e.1.entry(orig).or_default() += 1;
                }
            }
            let mut root_df: HashMap<&str, u32> = HashMap::new();
            for (st, root) in &canonical {
                *root_df.entry(root.as_str()).or_default() += df.get(st).copied().unwrap_or(0);
            }
            let n = with_ru.len() as f64;
            let forms: Vec<String> = term
                .ru
                .split(" / ")
                .chain(term.ru_plural.as_deref())
                .map(|f| parse::split_bracket(f).0)
                .collect();
            let mut variants: Vec<GameVariant> = counts
                .into_iter()
                .filter(|(st, (c, _))| {
                    let global = f64::from(
                        root_df
                            .get(st.as_str())
                            .copied()
                            .unwrap_or(*c)
                            .max(*c)
                            .max(1),
                    );
                    *c >= VARIANT_MIN_LINES
                        && f64::from(*c) >= VARIANT_MIN_SHARE * n
                        && f64::from(*c) / global >= VARIANT_MIN_PRECISION
                })
                .map(|(st, (c, spellings))| {
                    let ru = spellings
                        .into_iter()
                        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                        .map_or(st, |(o, _)| o);
                    let glossary = forms.iter().any(|f| matcher::has_form(&ru, f));
                    GameVariant {
                        ru,
                        lines: i32::try_from(c).unwrap_or(i32::MAX),
                        glossary,
                    }
                })
                .collect();
            variants.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.ru.cmp(&b.ru)));
            variants.truncate(VARIANTS_SHOWN);
            out.game_variants = variants;
            out.cn_game = speakers.get(&term.key.to_lowercase()).and_then(|by| {
                let total: u32 = by.values().sum();
                let (cn, k) = by
                    .iter()
                    .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))?;
                (*k >= 2 && f64::from(*k) >= 0.6 * f64::from(total)).then(|| cn.clone())
            });
            out
        })
        .collect()
}

/// Work out every term's [`TermStats`] over the story lines and store them.
pub async fn recount(pool: &PgPool) -> anyhow::Result<()> {
    let stored = load(pool).await?;
    if stored.is_empty() {
        return Ok(());
    }
    let rows: Vec<(String, Option<String>, String, Option<String>)> =
        sqlx::query_as("SELECT en, ru_game, speaker_en, speaker_cn FROM translators.story_lines")
            .fetch_all(pool)
            .await?;
    let ids: Vec<i32> = stored.iter().map(|s| s.id).collect();
    let all = tokio::task::spawn_blocking(move || {
        let terms: Vec<Term> = stored.iter().map(StoredTerm::as_term).collect();
        let lines: Vec<StatLine> = rows
            .into_iter()
            .map(|(en, ru_game, speaker_en, speaker_cn)| StatLine {
                en,
                ru_game,
                speaker_en,
                speaker_cn,
            })
            .collect();
        stats(&terms, &lines)
    })
    .await?;
    let counts: Vec<i32> = all.iter().map(|s| s.story_lines).collect();
    let checked: Vec<i32> = all.iter().map(|s| s.checked_lines).collect();
    let found: Vec<i32> = all.iter().map(|s| s.found_lines).collect();
    let variants: Vec<serde_json::Value> = all
        .iter()
        .map(|s| serde_json::to_value(&s.game_variants).unwrap_or_default())
        .collect();
    let cn: Vec<Option<String>> = all.iter().map(|s| s.cn_game.clone()).collect();
    sqlx::query(
        "UPDATE translators.glossary_terms g
         SET story_lines = c.n, checked_lines = c.checked, found_lines = c.found,
             game_variants = c.variants, cn_game = c.cn
         FROM UNNEST($1::int[], $2::int[], $3::int[], $4::int[], $5::jsonb[], $6::text[])
             AS c(id, n, checked, found, variants, cn)
         WHERE g.id = c.id",
    )
    .bind(&ids)
    .bind(&counts)
    .bind(&checked)
    .bind(&found)
    .bind(&variants)
    .bind(&cn)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod stats_tests {
    use super::*;

    fn term(key: &str, ru: &str) -> Term {
        Term {
            sheet: "Расы".into(),
            row: 2,
            en: key.into(),
            key: key.into(),
            qualifier: None,
            ru: ru.into(),
            ru_plural: None,
            note: None,
            synonyms: None,
            comment: None,
            cn: None,
            case_sensitive: true,
            from_sova: false,
        }
    }

    fn line(en: &str, ru: &str) -> StatLine {
        StatLine {
            en: en.into(),
            ru_game: Some(ru.into()),
            speaker_en: String::new(),
            speaker_cn: None,
        }
    }

    #[test]
    fn endings_of_one_word_are_one_variant() {
        let terms = vec![term("Infected", "Заражённый")];
        let lines = vec![
            line("The Infected.", "Заражённые."),
            line("An Infected.", "Заражённый."),
            line("Of the Infected.", "Из заражённых."),
        ];
        let s = &stats(&terms, &lines)[0];
        assert_eq!(s.game_variants.len(), 1, "{:?}", s.game_variants);
        assert_eq!(s.game_variants[0].lines, 3);
        assert!(s.game_variants[0].glossary);
    }

    #[test]
    fn variants_consistency_and_chinese() {
        let terms = vec![term("Ursus", "Урсин")];
        let mut lines = vec![
            line("The Ursus are here.", "Урсины здесь."),
            line("An Ursus soldier.", "Солдат урсин."),
            line("Back to Ursus.", "Назад в Урсус."),
            line("Ursus again.", "Снова Урсус, это урсус."),
            line("Nothing.", "Это ничего."),
            line("Something.", "Это что-то."),
        ];
        lines.push(StatLine {
            en: "Hi.".into(),
            ru_game: None,
            speaker_en: "Ursus".into(),
            speaker_cn: Some("乌萨斯".into()),
        });
        lines.push(StatLine {
            en: "Yo.".into(),
            ru_game: None,
            speaker_en: "Ursus".into(),
            speaker_cn: Some("乌萨斯".into()),
        });
        let s = &stats(&terms, &lines)[0];
        assert_eq!((s.story_lines, s.checked_lines, s.found_lines), (4, 4, 2));
        let v: Vec<(&str, i32, bool)> = s
            .game_variants
            .iter()
            .map(|v| (v.ru.as_str(), v.lines, v.glossary))
            .collect();
        assert_eq!(v, vec![("Урсины", 2, true), ("Урсус", 2, false)]);
        assert_eq!(s.cn_game.as_deref(), Some("乌萨斯"));
    }
}
