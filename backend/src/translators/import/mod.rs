//! The import: every EN story's displayed texts, with the Russian the game
//! shows now (ArknightsRU dictionaries + RichTextFix catalog) and the hints
//! from arktr (Sova, AI, CN), written to `translators.story_lines`.
//!
//! Re-runnable: the server's rows are replaced in one transaction, and the
//! source files are only read. Sources are named by environment variables so
//! that no machine path is committed:
//!
//! * `TRANSLATORS_RU_TEXT_DIR`: the `BepInEx/Translation/ru/Text` folder (required);
//! * `TRANSLATORS_PLUGIN_DLL`: `ArknightsLocalization.RichTextFix.dll` (optional);
//! * `TRANSLATORS_ARKTR_DIR`: the arktr checkout, read at `data/out` (optional).

pub mod arktr;
pub mod lines;
pub mod normalize;
pub mod plugin;
pub mod xunity;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use utoipa::ToSchema;

use crate::app::services::story::StoryRef;
use lines::{Line, Matcher};

/// Where the sources are, from the environment.
#[derive(Debug, Clone)]
pub struct Sources {
    pub ru_text_dir: PathBuf,
    pub plugin_dll: Option<PathBuf>,
    pub arktr_dir: Option<PathBuf>,
}

impl Sources {
    pub fn from_env() -> Result<Self, String> {
        let get = |k: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
        };
        let ru_text_dir =
            get("TRANSLATORS_RU_TEXT_DIR").ok_or("TRANSLATORS_RU_TEXT_DIR is not set")?;
        Ok(Self {
            ru_text_dir,
            plugin_dll: get("TRANSLATORS_PLUGIN_DLL"),
            arktr_dir: get("TRANSLATORS_ARKTR_DIR"),
        })
    }
}

/// One example of a text that found no match.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Unmatched {
    pub story_id: String,
    pub line: u32,
    pub en: String,
}

/// What an import read and attached.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub server: String,
    pub stories: usize,
    /// Stories listed by the library whose script file is missing.
    pub stories_without_script: usize,
    pub lines: usize,
    pub with_ru_game: usize,
    /// How the Russian was found: `exact`, `normalized`, `lines`, `plugin`.
    pub ru_game_by_match: HashMap<String, usize>,
    /// Lines whose text is in the plugin's `story` catalog (edits through the dictionaries may not show).
    pub plugin_story: usize,
    /// Lines the plugin deliberately leaves in English (`ambiguous:story`).
    pub plugin_ambiguous: usize,
    pub without_ru_game: usize,
    pub without_ru_game_examples: Vec<Unmatched>,
    pub with_sova: usize,
    pub with_ai: usize,
    pub with_cn: usize,
    /// Sova's chapter notes attached to stories (a note per story of its chapter).
    pub sova_notes: usize,
    pub dictionary_files: Vec<String>,
    pub dictionary_entries: usize,
    pub plugin_loaded: bool,
    /// arktr stories (scripts) found, and how many of them the library lists.
    pub arktr_stories: usize,
    pub arktr_stories_on_site: usize,
    pub arktr_rows_by_src_line: usize,
    pub arktr_rows_by_text: usize,
    pub arktr_rows_unattached: usize,
    pub arktr_unattached_examples: Vec<Unmatched>,
    pub millis: u128,
}

const EXAMPLES: usize = 10;

/// One story's id and its lines, in file order.
pub type StoryLines = (String, Vec<Line>);

/// Who a story sprite is (see [`lines::char_votes`]).
#[derive(Debug, Clone)]
pub struct CharName {
    pub char_id: String,
    pub name_en: String,
    pub name_ru: Option<String>,
    pub votes: u32,
    pub total: u32,
    /// Named by who talks after it, not by the game's character table.
    pub guessed: bool,
}

/// The import's output: every story's lines, the sprites' names, the summary.
pub struct Built {
    pub stories: Vec<StoryLines>,
    pub chars: Vec<CharName>,
    /// Sova's chapter notes by site story id.
    pub sova_notes: Vec<(String, Vec<String>)>,
    /// The library's chapter and story names with the game's Russian (`en`, `ru`).
    pub names_ru: Vec<(String, String)>,
    pub summary: ImportSummary,
}

/// Build every line of every story. Synchronous and CPU-bound: run it off the
/// async workers.
/// Operators by their number (`char_130_doberm` -> 130), for sprites.
pub struct OperatorIndex<'a> {
    by_number: HashMap<&'a str, Vec<(&'a str, &'a String)>>,
}

impl<'a> OperatorIndex<'a> {
    pub fn new(operators: &'a HashMap<String, String>) -> Self {
        let mut by_number: HashMap<&str, Vec<(&str, &String)>> = HashMap::new();
        for (id, name) in operators {
            let mut parts = id.splitn(3, '_');
            if let (Some(_), Some(num), Some(rest)) = (parts.next(), parts.next(), parts.next()) {
                by_number.entry(num).or_default().push((rest, name));
            }
        }
        Self { by_number }
    }
}

/// The operator a story sprite shows, if any: the same number and a sprite
/// name that starts with the operator's (`char_130_doberm_ex`,
/// `char_003_kaltsn07`, `avg_1037_amiya3` and `char_002_amiya_summer` are
/// Dobermann, Kal'tsit, Amiya and Amiya; `char_1002_nsabr` is nobody).
pub fn operator_of<'a>(sprite: &str, index: &OperatorIndex<'a>) -> Option<&'a String> {
    let mut parts = sprite.splitn(3, '_');
    let (_, num, rest) = (parts.next()?, parts.next()?, parts.next()?);
    index
        .by_number
        .get(num)?
        .iter()
        .filter(|(op, _)| rest.starts_with(op))
        .max_by_key(|(op, _)| op.len())
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod operator_tests {
    use super::*;

    #[test]
    fn sprites_find_their_operator() {
        let ops: HashMap<String, String> = [
            ("char_130_doberm", "Dobermann"),
            ("char_003_kalts", "Kal'tsit"),
            ("char_002_amiya", "Amiya"),
            ("char_1037_amiya3", "Amiya"),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .collect();
        let idx = OperatorIndex::new(&ops);
        let name = |s: &str| operator_of(s, &idx).map(String::as_str);
        assert_eq!(name("char_130_doberm_ex"), Some("Dobermann"));
        assert_eq!(name("char_003_kaltsn07"), Some("Kal'tsit"));
        assert_eq!(name("avg_003_kalts"), Some("Kal'tsit"));
        assert_eq!(name("avg_1037_amiya3"), Some("Amiya"));
        assert_eq!(name("char_002_amiya_summer"), Some("Amiya"));
        assert_eq!(name("char_002_amiya"), Some("Amiya"));
        assert_eq!(name("char_1002_nsabr"), None);
        assert_eq!(name("char_130_other"), None);
    }
}

/// What the game data gives the import besides the scripts.
#[derive(Debug, Default, Clone)]
pub struct GameNames {
    /// Operator id (`char_002_amiya`) -> English name, from the character table.
    pub operators: HashMap<String, String>,
    /// The library's chapter and story names, to look up in the dictionaries.
    pub library: Vec<String>,
}

pub fn build(
    sources: &Sources,
    assets_dir: &Path,
    stories: &[(String, StoryRef)],
    game: &GameNames,
) -> anyhow::Result<Built> {
    let started = std::time::Instant::now();
    let mut summary = ImportSummary {
        server: "en".into(),
        ..ImportSummary::default()
    };

    let dict = xunity::load(&sources.ru_text_dir)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", sources.ru_text_dir.display()))?;
    summary.dictionary_files.clone_from(&dict.files);
    summary.dictionary_entries = dict.entries.len();
    let plugin = match &sources.plugin_dll {
        Some(p) => Some(
            plugin::PluginCatalog::load(p)
                .map_err(|e| anyhow::anyhow!("reading {}: {e}", p.display()))?,
        ),
        None => None,
    };
    summary.plugin_loaded = plugin.is_some();
    let matcher = Matcher::new(&dict, plugin.as_ref());
    let arktr = match &sources.arktr_dir {
        Some(d) => arktr::load(d)?,
        None => arktr::Arktr::default(),
    };
    let arktr_rows = &arktr.rows;
    summary.arktr_stories = arktr_rows.len();
    let mut sova_notes = Vec::new();

    let mut out = Vec::with_capacity(stories.len());
    let mut seen_txt = HashSet::new();
    let mut votes: HashMap<String, HashMap<String, u32>> = HashMap::new();
    for (id, story_ref) in stories {
        let script = match crate::core::story::load_script(assets_dir, &story_ref.story_txt) {
            Ok(s) => s,
            Err(_) => {
                summary.stories_without_script += 1;
                continue;
            }
        };
        summary.stories += 1;
        let commands = crate::core::story::parser::parse(&script);
        lines::char_votes(&commands, &mut votes);
        let mut lines = lines::displayed(&commands);
        for l in &mut lines {
            matcher.apply(l);
        }
        if let Some(notes) = arktr.notes.get(&story_ref.story_txt) {
            sova_notes.push((id.clone(), notes.clone()));
        }
        if let Some(rows) = arktr_rows.get(&story_ref.story_txt) {
            if seen_txt.insert(story_ref.story_txt.clone()) {
                summary.arktr_stories_on_site += 1;
            }
            let (counts, missed) = lines::attach_arktr(&mut lines, rows);
            summary.arktr_rows_by_src_line += counts.by_src_line;
            summary.arktr_rows_by_text += counts.by_text;
            summary.arktr_rows_unattached += counts.unattached;
            for row in missed {
                if summary.arktr_unattached_examples.len() < EXAMPLES {
                    summary.arktr_unattached_examples.push(Unmatched {
                        story_id: id.clone(),
                        line: row.src_line,
                        en: if row.en.is_empty() {
                            format!("(CN) {}", row.cn)
                        } else {
                            row.en.clone()
                        },
                    });
                }
            }
        }
        for l in &lines {
            summary.lines += 1;
            if l.ru_game.is_some() {
                summary.with_ru_game += 1;
            } else {
                summary.without_ru_game += 1;
                // Spread the examples over the corpus instead of one story's first lines.
                if summary.without_ru_game_examples.len() < EXAMPLES
                    && summary.without_ru_game % 97 == 1
                {
                    summary.without_ru_game_examples.push(Unmatched {
                        story_id: id.clone(),
                        line: l.line,
                        en: l.en.clone(),
                    });
                }
            }
            if let Some(m) = l.ru_game_match {
                *summary.ru_game_by_match.entry(m.to_owned()).or_default() += 1;
            }
            match l.plugin_role.as_deref() {
                Some("story") => summary.plugin_story += 1,
                Some("ambiguous:story") => summary.plugin_ambiguous += 1,
                _ => {}
            }
            summary.with_sova += usize::from(l.sova.is_some());
            summary.with_ai += usize::from(l.ai.is_some());
            summary.with_cn += usize::from(l.cn.is_some());
        }
        out.push((id.clone(), lines));
    }
    let operators = OperatorIndex::new(&game.operators);
    let mut chars: Vec<CharName> = votes
        .into_iter()
        .filter_map(|(char_id, by)| {
            let total: u32 = by.values().sum();
            // An operator's sprite: its name from the character table, certain.
            if let Some(name) = operator_of(&char_id, &operators) {
                return Some(CharName {
                    name_ru: matcher.dict(name).map(|(v, _, _)| v),
                    name_en: name.clone(),
                    votes: by.get(name).copied().unwrap_or(0),
                    total,
                    guessed: false,
                    char_id,
                });
            }
            let (name, n) = by
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))?;
            // A clear majority of at least two lines, else the sprite stays unnamed.
            (n >= 2 && n * 2 >= total).then(|| CharName {
                name_ru: matcher.dict(&name).map(|(v, _, _)| v),
                char_id,
                name_en: name,
                votes: n,
                total,
                guessed: true,
            })
        })
        .collect();
    chars.sort_by(|a, b| a.char_id.cmp(&b.char_id));
    summary.millis = started.elapsed().as_millis();
    summary.sova_notes = sova_notes.iter().map(|(_, n)| n.len()).sum();
    let mut names_ru: Vec<(String, String)> = game
        .library
        .iter()
        .filter(|n| !n.trim().is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .filter_map(|en| {
            let (ru, _, how) = matcher.dict(en)?;
            (how != "lines" && ru != *en).then(|| (en.clone(), ru))
        })
        .collect();
    names_ru.sort();
    Ok(Built {
        stories: out,
        chars,
        sova_notes,
        names_ru,
        summary,
    })
}

/// One Russian per entry of the server's lines (`$1`): the same wherever the entry stands.
const ENTRY_RU: &str = "SELECT DISTINCT ON (edit_key) edit_key, ru_game FROM translators.story_lines \
     WHERE server = $1 AND ru_game IS NOT NULL ORDER BY edit_key, story_id, line, part";

/// Replace the server's rows and record the run, in one transaction. An
/// entry whose Russian changed keeps the old one in `ru_history`.
pub async fn store(
    pool: &PgPool,
    server: &str,
    started_at: chrono::DateTime<chrono::Utc>,
    built: &Built,
) -> Result<(), sqlx::Error> {
    let Built {
        stories,
        chars,
        sova_notes,
        names_ru,
        summary,
    } = built;
    // 20 binds per row, under Postgres's 65,535-parameter limit.
    const CHUNK: usize = 2_000;
    let mut tx = pool.begin().await?;
    // The Russian each entry has now, to tell afterwards which the import changed.
    sqlx::query(&format!(
        "CREATE TEMP TABLE _ru_before ON COMMIT DROP AS {ENTRY_RU}"
    ))
    .bind(server)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM translators.story_lines WHERE server = $1")
        .bind(server)
        .execute(&mut *tx)
        .await?;
    let rows: Vec<(&str, usize, &Line)> = stories
        .iter()
        .flat_map(|(id, lines)| {
            lines
                .iter()
                .enumerate()
                .map(move |(ord, l)| (id.as_str(), ord, l))
        })
        .collect();
    for chunk in rows.chunks(CHUNK) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO translators.story_lines (server, story_id, line, part, ord, kind, speaker_en, speaker_ru, en, \
             ru_game, ru_game_source, ru_game_match, plugin_role, ru_dict, speaker_cn, cn, sova, ai, arktr_match, edit_key) ",
        );
        qb.push_values(chunk, |mut b, (id, ord, l)| {
            b.push_bind(server)
                .push_bind(*id)
                .push_bind(i32::try_from(l.line).unwrap_or(i32::MAX))
                .push_bind(l.part)
                .push_bind(i32::try_from(*ord).unwrap_or(i32::MAX))
                .push_bind(l.kind)
                .push_bind(&l.speaker_en)
                .push_bind(&l.speaker_ru)
                .push_bind(&l.en)
                .push_bind(&l.ru_game)
                .push_bind(&l.ru_game_source)
                .push_bind(l.ru_game_match)
                .push_bind(&l.plugin_role)
                .push_bind(&l.ru_dict)
                .push_bind(&l.speaker_cn)
                .push_bind(&l.cn)
                .push_bind(&l.sova)
                .push_bind(&l.ai)
                .push_bind(l.arktr_match)
                .push_bind(&l.edit_key);
        });
        // Two texts on one line and part would be a parser change; keep the first.
        qb.push(" ON CONFLICT (server, story_id, line, part) DO NOTHING");
        qb.build().execute(&mut *tx).await?;
    }
    // Comparing versions: keep the Russian an entry had when the new one differs.
    sqlx::query(&format!(
        "INSERT INTO translators.ru_history (server, edit_key, text, changed_at)
         SELECT $1, b.edit_key, b.ru_game, $2
         FROM _ru_before b JOIN ({ENTRY_RU}) n ON n.edit_key = b.edit_key
         WHERE n.ru_game <> b.ru_game"
    ))
    .bind(server)
    .bind(started_at)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM translators.sova_notes")
        .execute(&mut *tx)
        .await?;
    let notes: Vec<(&str, i32, &str)> = sova_notes
        .iter()
        .flat_map(|(id, n)| {
            n.iter()
                .enumerate()
                .map(move |(i, t)| (id.as_str(), i32::try_from(i).unwrap_or(0), t.as_str()))
        })
        .collect();
    for chunk in notes.chunks(5_000) {
        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO translators.sova_notes (story_id, idx, text) ");
        qb.push_values(chunk, |mut b, (id, i, t)| {
            b.push_bind(*id).push_bind(*i).push_bind(*t);
        });
        qb.build().execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM translators.char_names")
        .execute(&mut *tx)
        .await?;
    for chunk in chars.chunks(5_000) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO translators.char_names (char_id, name_en, name_ru, votes, total, guessed) ",
        );
        qb.push_values(chunk, |mut b, c| {
            b.push_bind(&c.char_id)
                .push_bind(&c.name_en)
                .push_bind(&c.name_ru)
                .push_bind(i32::try_from(c.votes).unwrap_or(i32::MAX))
                .push_bind(i32::try_from(c.total).unwrap_or(i32::MAX))
                .push_bind(c.guessed);
        });
        qb.build().execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM translators.names_ru")
        .execute(&mut *tx)
        .await?;
    for chunk in names_ru.chunks(5_000) {
        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO translators.names_ru (en, ru) ");
        qb.push_values(chunk, |mut b, (en, ru)| {
            b.push_bind(en).push_bind(ru);
        });
        qb.build().execute(&mut *tx).await?;
    }
    sqlx::query(
        "INSERT INTO translators.imports (server, started_at, summary) VALUES ($1, $2, $3)",
    )
    .bind(server)
    .bind(started_at)
    .bind(sqlx::types::Json(summary))
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}
