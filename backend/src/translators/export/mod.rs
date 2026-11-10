//! The export to the game.
//!
//! The main export is the pack's own dictionaries corrected: `11_ui.txt`,
//! `12_story.txt`, `13_names.txt` with the approved values put in. Only the
//! changed files go out; in them only the value of a touched line changes —
//! its key as written, every other line, the order, the comments, the BOM and
//! the line breaks stay byte for byte (an export with no edits is the
//! original file). Entries no file has are added at the end of the
//! `12_story.txt` copy. Lines the RichTextFix plugin shows never go to a
//! dictionary: they go to `plugin_strings.tsv`. Every approved story line,
//! one row per place, goes to `story_lines.tsv` (the bundles); proposals for
//! one place go there only. `report.txt` lists each changed line: file, line
//! number, before → after, who approved. The `file` format writes a separate
//! `00_translators.txt` instead of the copies, "to check at home" with Alt+R.
//! Two modes: `approved` (for the game) and `review` (proposals, not
//! approved; REVIEW in the names).
//!
//! TSV: UTF-8 without BOM, LF, a header row; in a cell `\` -> `\\`, tab ->
//! `\t`, LF -> `\n`, CR -> `\r`.

pub mod encode;
pub mod zip;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::{IntoParams, ToSchema};

use super::SERVER;
use super::import::xunity::{decode_line, load_order};
use encode::{encode_line, encode_value};

/// The pack's dictionaries the main export corrects, in the pack's names.
pub const PATCHED: &[&str] = &["11_ui.txt", "12_story.txt", "13_names.txt"];

/// What to export.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ExportQuery {
    /// `approved` (for the game) or `review` (proposals, for checking).
    pub mode: String,
    /// `patch` (corrected copies of 11_ui / 12_story / 13_names, the main export) or
    /// `file` (one 00_translators.txt, to check at home).
    pub format: String,
    /// Empty: the report and the file list without contents; a file name: that file only.
    /// Left out: everything.
    #[serde(default)]
    pub only: Option<String>,
    /// The archive: the one prepared with the report (`archiveToken`); left
    /// out, it is built anew.
    #[serde(default)]
    pub token: Option<String>,
}

/// One file of an export.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportFile {
    pub name: String,
    /// Size in bytes (UTF-8).
    pub size: i32,
    pub content: String,
}

impl ExportFile {
    fn new(name: String, content: String) -> Self {
        Self {
            size: i32::try_from(content.len()).unwrap_or(i32::MAX),
            name,
            content,
        }
    }
}

/// A translation left out of the dictionaries, and why.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportSkipped {
    pub key: String,
    pub reason: String,
}

/// A dictionary file and how many of the exported keys it holds.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportFileKeys {
    pub file: String,
    pub keys: i32,
}

/// One line changed in a corrected copy.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportChange {
    pub file: String,
    /// 1-based line in the file; for an entry added at the end, its new line.
    pub line: i32,
    pub key: String,
    /// The value before; empty for an added entry.
    pub before: String,
    pub after: String,
    /// Who approved it (`approved`) or proposed it (`review`).
    pub by: String,
}

/// What an export wrote and what to know before putting it in the game.
#[derive(Debug, Clone, Default, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportReport {
    pub mode: String,
    pub format: String,
    /// Unix seconds.
    #[ts(type = "number")]
    pub at: i64,
    pub by: String,
    /// Lines changed or added in the dictionaries (the copies, or 00_translators.txt).
    pub written: i32,
    /// Each changed line of the copies.
    pub changes: Vec<ExportChange>,
    /// Left out: keys XUnity cannot read back.
    pub skipped: Vec<ExportSkipped>,
    /// Rows of `plugin_strings.tsv`.
    pub plugin_rows: i32,
    /// Rows of `story_lines.tsv`, and of them proposals for one place.
    pub story_rows: i32,
    pub context_rows: i32,
    /// `file` format: the files the game loads after ours, which win on the same key.
    pub loaded_after: Vec<String>,
    /// Of those, the ones holding exported keys, and how many.
    pub overridden_by: Vec<ExportFileKeys>,
    /// `patch` format: values replaced in each changed copy.
    pub replaced: Vec<ExportFileKeys>,
    /// `patch` format: entries none of the files had, added at the end of the 12_story copy.
    pub appended: i32,
    /// `review` mode: entries with more than one proposal (the most agreed is exported).
    pub with_alternatives: i32,
    /// The archive's name.
    pub archive: String,
    /// The list (`only` empty): the archive of exactly this export is being
    /// packed; download it with this token (for 30 minutes).
    pub archive_token: Option<String>,
}

/// The export's files and its report.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportResult {
    pub files: Vec<ExportFile>,
    pub report: ExportReport,
}

/// One entry to export.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Row {
    pub edit_key: String,
    pub text: String,
    pub ru_game: Option<String>,
    pub plugin_role: Option<String>,
    pub alternatives: i64,
    /// Who approved it (or proposed it, in review).
    pub by: String,
}

/// One proposal for one place.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContextRow {
    pub edit_key: String,
    pub line: i32,
    pub part: i16,
    pub story_id: String,
    pub ord: Option<i32>,
    pub en: Option<String>,
    pub ru_game: Option<String>,
    pub text: String,
    pub author: String,
    pub approved: bool,
}

/// A story line holding an exported entry.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Place {
    pub edit_key: String,
    pub story_id: String,
    pub line: i32,
    pub part: i16,
    pub en: String,
    pub ru_game: Option<String>,
}

/// The entries of a mode, by key.
pub async fn rows(pool: &PgPool, review: bool) -> Result<Vec<Row>, sqlx::Error> {
    let first_line = "LEFT JOIN LATERAL (
             SELECT s.ru_game, s.plugin_role FROM translators.story_lines s
             WHERE s.server = $1 AND s.edit_key = x.edit_key LIMIT 1) k ON TRUE";
    if review {
        sqlx::query_as(&format!(
            "SELECT x.edit_key, x.text, k.ru_game, k.plugin_role, x.alternatives,
                    COALESCE(NULLIF(u.nickname, ''), u.uid) AS by FROM (
                 SELECT DISTINCT ON (p.edit_key) p.edit_key, p.text, p.author_id,
                        COUNT(*) OVER (PARTITION BY p.edit_key) AS alternatives
                 FROM translators.proposals p
                 WHERE p.deleted_at IS NULL AND NOT p.context AND p.outdated_at IS NULL
                   AND NOT EXISTS (SELECT 1 FROM translators.approvals a
                                   WHERE a.edit_key = p.edit_key AND a.outdated_at IS NULL)
                 ORDER BY p.edit_key,
                          (SELECT COUNT(*) FROM translators.agreements g WHERE g.proposal_id = p.id) DESC,
                          p.created_at) x
             JOIN users u ON u.id = x.author_id
             {first_line}
             ORDER BY x.edit_key"
        ))
        .bind(SERVER)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as(&format!(
            "SELECT x.edit_key, x.text, k.ru_game, k.plugin_role, 1::bigint AS alternatives,
                    COALESCE(NULLIF(u.nickname, ''), u.uid) AS by
             FROM translators.approvals x
             JOIN users u ON u.id = x.approved_by
             {first_line}
             WHERE x.outdated_at IS NULL AND x.variant <> 'game'
             ORDER BY x.edit_key"
        ))
        .bind(SERVER)
        .fetch_all(pool)
        .await
    }
}

/// Proposals for one place: approved ones (`approved` mode, with the
/// approver) or the others (`review`, with the author).
pub async fn context_rows(pool: &PgPool, review: bool) -> Result<Vec<ContextRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.edit_key, p.line, p.part, p.story_id, s.ord, s.en, s.ru_game, p.text,
                CASE WHEN $2 THEN COALESCE(NULLIF(u.nickname, ''), u.uid)
                     ELSE COALESCE(NULLIF(v.nickname, ''), v.uid, '') END AS author,
                p.approved_at IS NOT NULL AS approved
         FROM translators.proposals p
         JOIN users u ON u.id = p.author_id
         LEFT JOIN users v ON v.id = p.approved_by
         LEFT JOIN translators.story_lines s
             ON s.server = $1 AND s.story_id = p.story_id AND s.line = p.line AND s.part = p.part
         WHERE p.deleted_at IS NULL AND p.context AND p.outdated_at IS NULL
           AND (p.approved_at IS NOT NULL) = NOT $2
         ORDER BY p.story_id, s.ord",
    )
    .bind(SERVER)
    .bind(review)
    .fetch_all(pool)
    .await
}

/// Every story line holding one of the keys.
pub async fn places(pool: &PgPool, keys: &[String]) -> Result<Vec<Place>, sqlx::Error> {
    sqlx::query_as(
        "SELECT edit_key, story_id, line, part, en, ru_game FROM translators.story_lines
         WHERE server = $1 AND edit_key = ANY($2) ORDER BY story_id, ord",
    )
    .bind(SERVER)
    .bind(keys)
    .fetch_all(pool)
    .await
}

/// A TSV cell: `\` -> `\\`, tab -> `\t`, LF -> `\n`, CR -> `\r`.
pub fn tsv_cell(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

/// A TSV file: UTF-8 without BOM, LF, a header row.
fn tsv(header: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = header.join("\t");
    out.push('\n');
    for r in rows {
        out.push_str(&r.iter().map(|c| tsv_cell(c)).collect::<Vec<_>>().join("\t"));
        out.push('\n');
    }
    out
}

/// Where the first unescaped `=` of a raw dictionary line stands (the
/// decoder's separator), in bytes.
fn separator(raw: &str) -> Option<usize> {
    let mut chars = raw.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '=' => return Some(i),
            _ => {}
        }
    }
    None
}

/// One dictionary file with the values of the touched keys replaced; every
/// other byte as it was. Returns the new text and the changed lines.
pub fn patch_file(
    name: &str,
    content: &str,
    entries: &BTreeMap<String, (String, String)>,
) -> (String, Vec<ExportChange>) {
    let mut out = String::with_capacity(content.len() + 1024);
    let mut changes = Vec::new();
    for (n, seg) in content.split_inclusive('\n').enumerate() {
        let (body, end) = match seg.strip_suffix("\r\n") {
            Some(b) => (b, "\r\n"),
            None => match seg.strip_suffix('\n') {
                Some(b) => (b, "\n"),
                None => (seg, ""),
            },
        };
        // A BOM at the very start belongs to the file, not to the key.
        let bare = body.strip_prefix('\u{feff}').unwrap_or(body);
        let new_line = decode_line(bare).and_then(|(k, old)| {
            let (value, by) = entries.get(&k)?;
            if *value == old {
                return None;
            }
            let at = separator(body)?;
            let line = format!("{}={}", &body[..at], encode_value(value).ok()?);
            // Never write a line that reads back otherwise.
            let check = line.strip_prefix('\u{feff}').unwrap_or(&line);
            (decode_line(check) == Some((k.clone(), value.clone()))).then(|| {
                changes.push(ExportChange {
                    file: name.to_owned(),
                    line: i32::try_from(n + 1).unwrap_or(i32::MAX),
                    key: k,
                    before: old,
                    after: value.clone(),
                    by: by.clone(),
                });
                line
            })
        });
        out.push_str(new_line.as_deref().unwrap_or(body));
        out.push_str(end);
    }
    (out, changes)
}

/// The export, from the rows and the pack's dictionaries folder. Pure apart
/// from reading that folder. `story_files` names each story's script file.
#[allow(clippy::too_many_arguments)]
pub fn build(
    rows: &[Row],
    context: &[ContextRow],
    places: &[Place],
    story_files: &HashMap<String, String>,
    review: bool,
    patch: bool,
    text_dir: &Path,
    by: &str,
    at: i64,
) -> std::io::Result<ExportResult> {
    let mut report = ExportReport {
        mode: if review { "review" } else { "approved" }.to_owned(),
        format: if patch { "patch" } else { "file" }.to_owned(),
        at,
        by: by.to_owned(),
        ..ExportReport::default()
    };
    let tag = if review { "REVIEW_" } else { "" };
    let when = chrono::DateTime::from_timestamp(at, 0)
        .map(|t| t.format("%Y-%m-%d_%H%M").to_string())
        .unwrap_or_default();
    report.archive = format!(
        "translators_{}{}.zip",
        if review { "REVIEW_" } else { "" },
        when
    );
    let mut files = Vec::new();

    // Plugin lines apart; unchanged texts dropped; unwritable keys reported.
    let mut entries: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut by_key: HashMap<&str, &Row> = HashMap::new();
    let mut plugin: Vec<Vec<String>> = Vec::new();
    for r in rows {
        if r.ru_game.as_deref() == Some(r.text.as_str()) {
            continue;
        }
        by_key.insert(r.edit_key.as_str(), r);
        report.with_alternatives += i32::from(r.alternatives > 1);
        if r.plugin_role.as_deref() == Some("story") {
            plugin.push(vec![
                r.edit_key.clone(),
                r.ru_game.clone().unwrap_or_default(),
                r.text.clone(),
                r.plugin_role.clone().unwrap_or_default(),
            ]);
            continue;
        }
        match encode_line(&r.edit_key, &r.text) {
            Ok(_) => {
                entries.insert(r.edit_key.clone(), (r.text.clone(), r.by.clone()));
            }
            Err(e) => report.skipped.push(ExportSkipped {
                key: r.edit_key.clone(),
                reason: e.as_str().to_owned(),
            }),
        }
    }

    let ours = if review {
        "00_translators_REVIEW.txt"
    } else {
        "00_translators.txt"
    };
    if patch {
        let mut left: BTreeMap<String, (String, String)> = entries.clone();
        for name in PATCHED {
            let Ok(content) = std::fs::read_to_string(text_dir.join(name)) else {
                continue;
            };
            let (mut body, changes) = patch_file(name, &content, &entries);
            for c in &changes {
                left.remove(&c.key);
            }
            let mut replaced = changes.len();
            report.changes.extend(changes);
            if *name == "12_story.txt" && !left.is_empty() {
                // Entries no file has: added at the end, in the file's own line breaks.
                let nl = if content.contains("\r\n") {
                    "\r\n"
                } else {
                    "\n"
                };
                if !body.is_empty() && !body.ends_with('\n') {
                    body.push_str(nl);
                }
                let mut line = body.split_inclusive('\n').count();
                for (k, (v, who)) in &left {
                    body.push_str(&encode_line(k, v).unwrap_or_default());
                    body.push_str(nl);
                    line += 1;
                    report.changes.push(ExportChange {
                        file: (*name).to_owned(),
                        line: i32::try_from(line).unwrap_or(i32::MAX),
                        key: k.clone(),
                        before: String::new(),
                        after: v.clone(),
                        by: who.clone(),
                    });
                }
                report.appended = i32::try_from(left.len()).unwrap_or(i32::MAX);
                replaced += left.len();
            }
            if replaced > 0 {
                report.replaced.push(ExportFileKeys {
                    file: (*name).to_owned(),
                    keys: i32::try_from(replaced).unwrap_or(i32::MAX),
                });
                files.push(ExportFile::new(format!("{tag}{name}"), body));
            }
        }
        report.written = report.replaced.iter().map(|r| r.keys).sum();
    } else {
        let when = chrono::DateTime::from_timestamp(at, 0)
            .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_default();
        let mut body = String::new();
        if review {
            body.push_str("// REVIEW — НЕ ДЛЯ ИГРЫ БЕЗ ПРОВЕРКИ: неутверждённые предложения\n");
        }
        body.push_str(&format!(
            "// ArknightsRU — правки раздела «Для переводчиков» (KaltsitWebsite), ДЛЯ ПРОВЕРКИ У СЕБЯ\n\
             // Положите файл в BepInEx\\Translation\\ru\\Text и нажмите в игре Alt+R (или перезапустите игру).\n\
             // Выгружено: {when} · строк: {} · выгрузил(а): {by}\n",
            entries.len()
        ));
        for (k, (v, _)) in &entries {
            body.push_str(&encode_line(k, v).unwrap_or_default());
            body.push('\n');
        }
        report.written = i32::try_from(entries.len()).unwrap_or(i32::MAX);
        files.push(ExportFile::new(ours.to_owned(), body));

        // What the game loads after our file (kit §3.3: by full path, descending,
        // case-insensitive; a later file wins on the same key).
        let ours_path = text_dir.join(ours).to_string_lossy().to_lowercase();
        let mut keys_after: HashMap<String, i32> = HashMap::new();
        for path in load_order(text_dir).unwrap_or_default() {
            let lower = path.to_string_lossy().to_lowercase();
            let name = path
                .strip_prefix(text_dir)
                .map_or_else(|_| lower.clone(), |p| p.to_string_lossy().into_owned());
            if lower >= ours_path || name.eq_ignore_ascii_case(ours) {
                continue;
            }
            report.loaded_after.push(name.clone());
            let count = std::fs::read_to_string(&path)
                .map(|c| {
                    c.split(['\r', '\n'])
                        .filter_map(decode_line)
                        .filter(|(k, _)| entries.contains_key(k))
                        .count()
                })
                .unwrap_or(0);
            if count > 0 {
                keys_after.insert(name, i32::try_from(count).unwrap_or(i32::MAX));
            }
        }
        report.overridden_by = report
            .loaded_after
            .iter()
            .filter_map(|f| {
                keys_after.get(f).map(|&keys| ExportFileKeys {
                    file: f.clone(),
                    keys,
                })
            })
            .collect();
    }

    // story_lines.tsv: every exported story line at each of its places; a
    // proposal for one place replaces the entry's text there.
    let mut story: BTreeMap<(String, i32, i16), Vec<String>> = BTreeMap::new();
    let file_of = |id: &str| {
        story_files
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_owned())
    };
    let line_no = |line: i32, part: i16| {
        if part > 0 {
            format!("{line}.{part}")
        } else {
            line.to_string()
        }
    };
    for p in places {
        let Some(r) = by_key.get(p.edit_key.as_str()) else {
            continue;
        };
        if p.ru_game.as_deref() == Some(r.text.as_str()) {
            continue;
        }
        story.insert(
            (p.story_id.clone(), p.line, p.part),
            vec![
                file_of(&p.story_id),
                line_no(p.line, p.part),
                p.en.clone(),
                p.ru_game.clone().unwrap_or_default(),
                r.text.clone(),
            ],
        );
    }
    for c in context {
        story.insert(
            (c.story_id.clone(), c.line, c.part),
            vec![
                file_of(&c.story_id),
                line_no(c.line, c.part),
                c.en.clone().unwrap_or_default(),
                c.ru_game.clone().unwrap_or_default(),
                c.text.clone(),
            ],
        );
    }
    report.context_rows = i32::try_from(context.len()).unwrap_or(i32::MAX);
    report.story_rows = i32::try_from(story.len()).unwrap_or(i32::MAX);
    if !story.is_empty() {
        let rows: Vec<Vec<String>> = story.into_values().collect();
        files.push(ExportFile::new(
            format!("{tag}story_lines.tsv"),
            tsv(
                &["файл сюжета", "строка", "английский", "было", "стало"],
                &rows,
            ),
        ));
    }
    report.plugin_rows = i32::try_from(plugin.len()).unwrap_or(i32::MAX);
    if !plugin.is_empty() {
        files.push(ExportFile::new(
            format!("{tag}plugin_strings.tsv"),
            tsv(&["английский", "было", "стало", "роль"], &plugin),
        ));
    }
    files.push(ExportFile::new(
        format!("{tag}report.txt"),
        report_text(&report, ours),
    ));
    Ok(ExportResult { files, report })
}

/// The report as a text file in the archive.
fn report_text(r: &ExportReport, ours: &str) -> String {
    let mut out = String::new();
    let when = chrono::DateTime::from_timestamp(r.at, 0)
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_default();
    out.push_str(&format!(
        "Выгрузка раздела «Для переводчиков»\nРежим: {}\nФормат: {}\nКогда: {when}\nКто выгрузил: {}\n\n",
        if r.mode == "review" { "все предложения — на проверку (REVIEW, не для игры)" } else { "утверждённое — для игры" },
        if r.format == "patch" { "исправленные копии 11_ui / 12_story / 13_names (в архиве — только изменённые)" } else { "отдельный файл для проверки у себя" },
        r.by
    ));
    if r.format == "patch" {
        out.push_str(&format!("Изменено строк в словарях: {}\n", r.written));
        for f in &r.replaced {
            out.push_str(&format!("  {}: {}\n", f.file, f.keys));
        }
        if r.appended > 0 {
            out.push_str(&format!(
                "  из них новых записей в конце 12_story.txt: {}\n",
                r.appended
            ));
        }
        out.push_str("\nИзменённые строки (файл:строка — было → стало — кто):\n");
        for c in &r.changes {
            let before = if c.before.is_empty() {
                "(новая запись)".to_owned()
            } else {
                tsv_cell(&c.before)
            };
            out.push_str(&format!(
                "  {}:{} — {} → {} — {}\n    ключ: {}\n",
                c.file,
                c.line,
                before,
                tsv_cell(&c.after),
                c.by,
                tsv_cell(&c.key)
            ));
        }
    } else {
        out.push_str(&format!("Записано строк в {ours}: {}\n", r.written));
    }
    if r.mode == "review" {
        out.push_str(&format!(
            "Записей с несколькими предложениями (взято с наибольшим «согласен»): {}\n",
            r.with_alternatives
        ));
    }
    out.push_str(&format!(
        "\nСтроки плагина RichTextFix (plugin_strings.tsv): {}\n",
        r.plugin_rows
    ));
    out.push_str(&format!(
        "Строки сюжета для бандлов (story_lines.tsv): {} (из них «только для этого места»: {})\n",
        r.story_rows, r.context_rows
    ));
    out.push_str(&format!(
        "Не записано (XUnity не прочтёт): {}\n",
        r.skipped.len()
    ));
    for s in &r.skipped {
        out.push_str(&format!("  {} — {}\n", tsv_cell(&s.key), s.reason));
    }
    if r.format == "file" {
        out.push_str(
            "\nКак проверить в игре: положите файл в BepInEx\\Translation\\ru\\Text и нажмите в игре Alt+R (или перезапустите игру).\n",
        );
        out.push_str(
            "\nФайлы, которые игра грузит ПОСЛЕ нашего (при одинаковом ключе побеждают они):\n",
        );
        if r.loaded_after.is_empty() {
            out.push_str("  нет — наш файл побеждает\n");
        }
        for f in &r.loaded_after {
            let keys = r
                .overridden_by
                .iter()
                .find(|o| &o.file == f)
                .map_or(0, |o| o.keys);
            out.push_str(&format!("  {f} — перекрывает наших ключей: {keys}\n"));
        }
    } else {
        out.push_str(
            "\nКак проверить в игре: замените файлы в BepInEx\\Translation\\ru\\Text исправленными копиями (оригиналы сохраните) и нажмите в игре Alt+R (или перезапустите игру).\n",
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn row(k: &str, v: &str, ru: Option<&str>, plugin: bool) -> Row {
        Row {
            edit_key: k.into(),
            text: v.into(),
            ru_game: ru.map(str::to_owned),
            plugin_role: plugin.then(|| "story".to_owned()),
            alternatives: 1,
            by: "lead".into(),
        }
    }

    fn sha(s: &str) -> String {
        hex::encode(Sha256::digest(s.as_bytes()))
    }

    /// A dictionary as the pack writes them, with the awkward bits: comments,
    /// escapes, a comment after the value, CRLF or LF, a BOM, no final break.
    fn pack(crlf: bool, bom: bool) -> String {
        let nl = if crlf { "\r\n" } else { "\n" };
        let lines = [
            "// ArknightsRU",
            "// comment",
            r"Hi=Привет",
            r"a\=b=в\=г",
            r"Line one\nLine two=Строка один\nСтрока два",
            r"x\/y=z\t",
            "Bye=Пока // tail",
            "#Boxing #MakingFriends=#Бокс",
        ];
        let mut s = lines.join(nl);
        if bom {
            s.insert(0, '\u{feff}');
        }
        s
    }

    #[test]
    fn no_edits_is_the_original_byte_for_byte() {
        for crlf in [false, true] {
            for bom in [false, true] {
                let orig = pack(crlf, bom);
                let (out, changes) = patch_file("12_story.txt", &orig, &BTreeMap::new());
                assert_eq!(sha(&out), sha(&orig), "crlf={crlf} bom={bom}");
                assert!(changes.is_empty());
            }
        }
    }

    #[test]
    fn one_edit_changes_exactly_one_line() {
        for crlf in [false, true] {
            let orig = pack(crlf, true);
            let entries = BTreeMap::from([(
                "a=b".to_owned(),
                ("в=г!\nдва".to_owned(), "lead".to_owned()),
            )]);
            let (out, changes) = patch_file("12_story.txt", &orig, &entries);
            let a: Vec<&str> = orig.split_inclusive('\n').collect();
            let b: Vec<&str> = out.split_inclusive('\n').collect();
            assert_eq!(a.len(), b.len());
            let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
            assert_eq!(diff, vec![3], "only line 4 differs");
            // The key part kept as written; the line break kept.
            assert!(b[3].starts_with(r"a\=b=") && b[3].ends_with(if crlf { "\r\n" } else { "\n" }));
            assert_eq!(
                decode_line(b[3].trim_end_matches(['\r', '\n'])),
                Some(("a=b".into(), "в=г!\nдва".into()))
            );
            assert_eq!(changes[0].line, 4);
            assert_eq!(changes[0].before, "в=г");
            assert!(out.starts_with('\u{feff}'));
        }
    }

    #[test]
    fn patch_export_keeps_untouched_files_out_and_appends_new_keys() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("11_ui.txt"), "// h\r\nOK=Ок\r\n").unwrap();
        std::fs::write(
            dir.path().join("12_story.txt"),
            "// h\r\nHi=Привет\r\nBye=Пока\r\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("13_names.txt"), "Amiya=Амия\n").unwrap();
        let rows = vec![
            row("Hi", "Здравствуй", Some("Привет"), false),
            row("New", "Новое", None, false),
            row("Plugin\tline", "Новое\nдва", Some("Старое"), true),
        ];
        let places = vec![Place {
            edit_key: "Hi".into(),
            story_id: "main_0_x".into(),
            line: 12,
            part: 0,
            en: "Hi".into(),
            ru_game: Some("Привет".into()),
        }];
        let ctx = vec![ContextRow {
            edit_key: "Bye".into(),
            line: 20,
            part: 2,
            story_id: "main_0_x".into(),
            ord: Some(5),
            en: Some("Bye".into()),
            ru_game: Some("Пока".into()),
            text: "До встречи".into(),
            author: "lead".into(),
            approved: true,
        }];
        let files_map = HashMap::from([(
            "main_0_x".to_owned(),
            "obt/main/level_main_00-01_end".to_owned(),
        )]);
        let out = build(
            &rows,
            &ctx,
            &places,
            &files_map,
            false,
            true,
            dir.path(),
            "dev",
            0,
        )
        .unwrap();
        let names: Vec<&str> = out.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "12_story.txt",
                "story_lines.tsv",
                "plugin_strings.tsv",
                "report.txt"
            ]
        );
        let story = &out.files[0].content;
        assert_eq!(story, "// h\r\nHi=Здравствуй\r\nBye=Пока\r\nNew=Новое\r\n");
        assert_eq!(out.report.appended, 1);
        let tsv_story = &out.files[1].content;
        assert_eq!(
            tsv_story,
            "файл сюжета\tстрока\tанглийский\tбыло\tстало\nobt/main/level_main_00-01_end\t12\tHi\tПривет\tЗдравствуй\nobt/main/level_main_00-01_end\t20.2\tBye\tПока\tДо встречи\n"
        );
        let tsv_plugin = &out.files[2].content;
        assert_eq!(
            tsv_plugin,
            "английский\tбыло\tстало\tроль\nPlugin\\tline\tСтарое\tНовое\\nдва\tstory\n"
        );
        let report = &out.files[3].content;
        assert!(report.contains("12_story.txt:2 — Привет → Здравствуй — lead"));
        assert!(
            !out.files.iter().any(|f| f.name == "11_ui.txt"),
            "untouched copies stay out"
        );
    }

    #[test]
    fn file_format_is_to_check_at_home() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("00_arktr_story.txt"), "Hi=Здорово\n").unwrap();
        let rows = vec![
            row("Hi", "Здравствуй", Some("Привет"), false),
            row("r:^x$", "y", None, false),
        ];
        let out = build(
            &rows,
            &[],
            &[],
            &HashMap::new(),
            false,
            false,
            dir.path(),
            "dev",
            0,
        )
        .unwrap();
        let file = &out.files[0];
        assert_eq!(file.name, "00_translators.txt");
        assert!(file.content.contains("ДЛЯ ПРОВЕРКИ У СЕБЯ") && file.content.contains("Alt+R"));
        assert!(file.content.ends_with("Hi=Здравствуй\n"));
        assert_eq!(out.report.skipped[0].key, "r:^x$");
        assert_eq!(out.report.loaded_after, vec!["00_arktr_story.txt"]);
        let report = &out.files.last().unwrap().content;
        assert!(report.contains("BepInEx\\Translation\\ru\\Text") && report.contains("Alt+R"));
    }

    /// The pack's real dictionaries (read only): no edits gives each file
    /// byte for byte; one edit changes exactly one line.
    #[test]
    #[ignore = "reads the pack's dictionaries from TRANSLATORS_RU_TEXT_DIR"]
    fn the_pack_files_round_trip() {
        let dir = std::path::PathBuf::from(
            std::env::var("TRANSLATORS_RU_TEXT_DIR").expect("TRANSLATORS_RU_TEXT_DIR"),
        );
        for name in PATCHED {
            let orig = std::fs::read_to_string(dir.join(name)).unwrap();
            let (same, none) = patch_file(name, &orig, &BTreeMap::new());
            assert_eq!(sha(&same), sha(&orig), "{name}: no edits");
            assert!(none.is_empty());
            println!(
                "{name}: sha256 {} = original; CRLF lines {}; BOM {}",
                sha(&same),
                orig.matches("\r\n").count(),
                orig.starts_with('\u{feff}')
            );
            // One edit: the first translation in the file.
            let (key, value) = orig
                .split_inclusive('\n')
                .find_map(|l| decode_line(l.trim_end_matches(['\r', '\n'])))
                .unwrap();
            let entries =
                BTreeMap::from([(key.clone(), (format!("{value} [тест]"), "lead".to_owned()))]);
            let (one, changes) = patch_file(name, &orig, &entries);
            let a: Vec<&str> = orig.split_inclusive('\n').collect();
            let b: Vec<&str> = one.split_inclusive('\n').collect();
            assert_eq!(a.len(), b.len());
            let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
            assert_eq!(diff.len(), 1, "{name}: one line");
            let i = diff[0];
            println!("{name}: diff line {}\n- {:?}\n+ {:?}", i + 1, a[i], b[i]);
            assert_eq!(changes.len(), 1);
            assert_eq!(usize::try_from(changes[0].line).unwrap(), i + 1);
        }
    }

    #[test]
    fn tsv_cells_escape_tabs_and_breaks() {
        assert_eq!(tsv_cell("a\tb\nc\r\\d"), "a\\tb\\nc\\r\\\\d");
    }
}

/// A pack version and what went out in it.
#[derive(Debug, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Release {
    #[ts(type = "number")]
    pub id: i64,
    pub version: String,
    pub note: Option<String>,
    pub by: Option<String>,
    #[ts(type = "number")]
    pub at: i64,
    /// Entries marked: dictionary lines, plugin lines, one-place lines.
    pub dictionary: i32,
    pub plugin: i32,
    pub context: i32,
}

/// Mark a pack version: which version, and a note.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInput {
    pub version: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Mark every approved entry for the game that has not gone out with the same
/// text as gone out in `version` (dictionary, plugin and one-place lines).
/// Returns the release with its counts; nothing new is still a release (0).
pub async fn mark_release(
    pool: &PgPool,
    version: &str,
    note: Option<&str>,
    by: uuid::Uuid,
) -> Result<i64, sqlx::Error> {
    let rows = rows(pool, false).await?;
    let context = context_rows(pool, false).await?;
    let mut tx = pool.begin().await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO translators.releases (version, note, released_by) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(version)
    .bind(note)
    .bind(by)
    .fetch_one(&mut *tx)
    .await?;
    let new_item = "INSERT INTO translators.release_items (release_id, edit_key, text, kind, story_id, line, part)
         SELECT $1, $2, $3, $4, $5, $6, $7
         WHERE NOT EXISTS (SELECT 1 FROM translators.release_items i
                           WHERE i.edit_key = $2 AND i.text = $3
                             AND i.story_id IS NOT DISTINCT FROM $5 AND i.line IS NOT DISTINCT FROM $6
                             AND i.part IS NOT DISTINCT FROM $7)";
    for r in &rows {
        if r.ru_game.as_deref() == Some(r.text.as_str()) {
            continue;
        }
        let kind = if r.plugin_role.as_deref() == Some("story") {
            "plugin"
        } else {
            "dictionary"
        };
        sqlx::query(new_item)
            .bind(id)
            .bind(&r.edit_key)
            .bind(&r.text)
            .bind(kind)
            .bind(None::<String>)
            .bind(None::<i32>)
            .bind(None::<i16>)
            .execute(&mut *tx)
            .await?;
    }
    for c in &context {
        sqlx::query(new_item)
            .bind(id)
            .bind(&c.edit_key)
            .bind(&c.text)
            .bind("context")
            .bind(&c.story_id)
            .bind(c.line)
            .bind(c.part)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// Every pack version marked, newest first, with its counts.
pub async fn releases(pool: &PgPool) -> Result<Vec<Release>, sqlx::Error> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        i64,
        String,
        Option<String>,
        Option<String>,
        chrono::DateTime<chrono::Utc>,
        i64,
        i64,
        i64,
    )> = sqlx::query_as(
        "SELECT r.id, r.version, r.note, COALESCE(NULLIF(u.nickname, ''), u.uid), r.released_at,
                    COUNT(*) FILTER (WHERE i.kind = 'dictionary'),
                    COUNT(*) FILTER (WHERE i.kind = 'plugin'),
                    COUNT(*) FILTER (WHERE i.kind = 'context')
             FROM translators.releases r
             LEFT JOIN users u ON u.id = r.released_by
             LEFT JOIN translators.release_items i ON i.release_id = r.id
             GROUP BY r.id, u.nickname, u.uid ORDER BY r.id DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, version, note, by, at, d, p, c)| Release {
            id,
            version,
            note,
            by,
            at: at.timestamp(),
            dictionary: i32::try_from(d).unwrap_or(i32::MAX),
            plugin: i32::try_from(p).unwrap_or(i32::MAX),
            context: i32::try_from(c).unwrap_or(i32::MAX),
        })
        .collect())
}
