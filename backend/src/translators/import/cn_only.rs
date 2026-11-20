//! The stories the CN client has and the EN one has not yet ("Только в
//! китайской версии": an event before its Global release). Their lines come
//! from the CN scripts; the Russian hints from arktr, which translates such
//! stories from the Chinese: the machine's text (`ai_cn`) and the user's own
//! corrections of it (`user_edit`, shown as Артём's variant with the
//! machine's text before it as the machine's).
//!
//! A line has no English, so no dictionary entry: it is its own entry,
//! `cn:<story id>#<line>.<part>`. Such entries never reach the export (the
//! game's dictionaries are keyed by the English, which does not exist yet).
//! When the EN client gets the story (the library lists the same story id),
//! [`transfer`] moves what was said about each line onto the EN line paired
//! with its CN line (`cn.rs`), or leaves it for a check by hand.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;
use sqlx::{Postgres, Transaction};

use super::arktr::Arktr;
use super::cn::{self, How};
use super::lines::Line;
use super::{CnPair, StoryLines};

/// One story only the CN client has.
#[derive(Debug, Clone)]
pub struct CnStory {
    pub story_id: String,
    pub story_txt: String,
    pub group_id: String,
    pub group_cn: String,
    pub group_ru: Option<String>,
    pub name_cn: String,
    pub name_ru: Option<String>,
    /// «До операции», «После операции»… (arktr's Russian, else the CN tag).
    pub avg_tag: Option<String>,
    pub code: Option<String>,
    pub sort: i32,
    pub lines: Vec<Line>,
}

/// The entry of a CN-only line.
pub fn key(story_id: &str, line: u32, part: i16) -> String {
    format!("cn:{story_id}#{line}.{part}")
}

/// `(story id, line, part)` of a CN-only entry.
pub fn parse_key(key: &str) -> Option<(&str, u32, i16)> {
    let rest = key.strip_prefix("cn:")?;
    let (story, place) = rest.rsplit_once('#')?;
    let (line, part) = place.split_once('.')?;
    Some((story, line.parse().ok()?, part.parse().ok()?))
}

fn text<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
}

/// Every story of the CN library the EN one lacks, with its lines and arktr's Russian.
pub fn collect(cn_dir: &Path, en_ids: &HashSet<&str>, arktr: &Arktr) -> Vec<CnStory> {
    let Some(table) = std::fs::read(cn::excel(cn_dir).join("story_review_table.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for g in table
        .get("Story_reviews")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let (Some(group_id), Some(v)) = (g.get("key").and_then(Value::as_str), g.get("value"))
        else {
            continue;
        };
        let group_cn = text(v, "Name").unwrap_or(group_id).to_owned();
        for d in v
            .get("InfoUnlockDatas")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(story_id), Some(story_txt)) = (text(d, "StoryId"), text(d, "StoryTxt"))
            else {
                continue;
            };
            if en_ids.contains(story_id) {
                continue;
            }
            let Some(mut lines) = cn::script_lines(cn_dir, story_txt) else {
                continue;
            };
            for l in &mut lines {
                // The CN text stands where the parser put it: in the English's place.
                l.cn = Some(std::mem::take(&mut l.en));
                // The speaker's CN name stands in the EN name's place too (under arktr's Russian one).
                l.speaker_cn = (!l.speaker_en.is_empty()).then(|| l.speaker_en.clone());
                l.edit_key = key(story_id, l.line, l.part);
            }
            attach(&mut lines, story_txt, arktr);
            let (name_ru, tag_ru) = arktr
                .story_ru
                .get(story_txt)
                .cloned()
                .unwrap_or_default();
            let non_empty = |s: String| (!s.trim().is_empty()).then_some(s);
            out.push(CnStory {
                story_id: story_id.to_owned(),
                story_txt: story_txt.to_owned(),
                group_id: group_id.to_owned(),
                group_cn: group_cn.clone(),
                group_ru: arktr.group_ru.get(group_id).cloned(),
                name_cn: text(d, "StoryName").unwrap_or(story_id).to_owned(),
                name_ru: non_empty(name_ru),
                avg_tag: non_empty(tag_ru).or_else(|| text(d, "AvgTag").map(str::to_owned)),
                code: text(d, "StoryCode").map(str::to_owned),
                sort: text(d, "StorySort")
                    .and_then(|s| s.parse().ok())
                    .or_else(|| d.get("StorySort").and_then(Value::as_i64).and_then(|n| i32::try_from(n).ok()))
                    .unwrap_or(0),
                lines,
            });
        }
    }
    out.sort_by(|a, b| (&a.group_id, a.sort, &a.story_id).cmp(&(&b.group_id, b.sort, &b.story_id)));
    out
}

/// arktr's rows at the same CN line: the speaker's Russian name, the machine's
/// text, and the user's correction as the second translator's variant.
fn attach(lines: &mut [Line], story_txt: &str, arktr: &Arktr) {
    let Some(rows) = arktr.rows.get(story_txt) else {
        return;
    };
    let at: HashMap<(u32, i16), usize> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| ((l.line, l.part), i))
        .collect();
    for row in rows {
        let part = if row.kind == "option" {
            row.opt.map_or(0, |o| i16::try_from(o).unwrap_or(0))
        } else {
            0
        };
        let Some(&i) = at.get(&(row.src_line, part)) else {
            continue;
        };
        let l = &mut lines[i];
        if !row.speaker_ru.trim().is_empty() {
            l.speaker_ru = Some(row.speaker_ru.clone());
        }
        l.arktr_match = Some("src_line");
        if let Some(own) = row.user() {
            l.artem = Some(own.to_owned());
            l.ai = row
                .line_idx
                .and_then(|i| arktr.before_user.get(&format!("{story_txt}#{i}")))
                .cloned();
        } else {
            l.ai = row.ai().map(str::to_owned);
        }
    }
}

/// What the import did with the edits of CN-only lines whose story the EN client now has.
#[derive(Debug, Default, Clone, Copy)]
pub struct Transferred {
    pub moved: usize,
    pub manual: usize,
}

/// Move the edits of the CN-only entries whose story the EN client now has
/// onto the EN lines, inside the import's transaction (before its lines are
/// replaced). An entry moves when its CN line pairs with an EN line in order or
/// at the same line number, nobody has said anything about the EN entry yet, and
/// — when it is approved — the EN entry stands nowhere else; otherwise it stays
/// as it is, listed `manual` ("check by hand").
pub async fn transfer(
    tx: &mut Transaction<'_, Postgres>,
    stories: &[StoryLines],
    pairs: &HashMap<String, Vec<CnPair>>,
) -> Result<Transferred, sqlx::Error> {
    let mut done = Transferred::default();
    let keys: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT k.edit_key, (SELECT s.cn FROM translators.story_lines s WHERE s.edit_key = k.edit_key LIMIT 1)
         FROM (SELECT edit_key FROM translators.proposals WHERE deleted_at IS NULL
               UNION SELECT edit_key FROM translators.approvals
               UNION SELECT edit_key FROM translators.comments WHERE deleted_at IS NULL
               UNION SELECT edit_key FROM translators.agreements) k
         WHERE k.edit_key LIKE 'cn:%'
           AND NOT EXISTS (SELECT 1 FROM translators.cn_transfers t WHERE t.old_key = k.edit_key AND t.state = 'moved')",
    )
    .fetch_all(&mut **tx)
    .await?;
    if keys.is_empty() {
        return Ok(done);
    }
    let by_story: HashMap<&str, &Vec<Line>> =
        stories.iter().map(|(id, l)| (id.as_str(), l)).collect();
    let mut uses: HashMap<&str, usize> = HashMap::new();
    for (_, lines) in stories {
        for l in lines {
            *uses.entry(l.edit_key.as_str()).or_default() += 1;
        }
    }
    for (old, cn_text) in keys {
        let Some((story, cn_line, cn_part)) = parse_key(&old) else {
            continue;
        };
        let Some(lines) = by_story.get(story) else {
            continue; // Still CN only.
        };
        let target = pairs
            .get(story)
            .and_then(|p| p.iter().find(|p| p.cn == (cn_line, cn_part)))
            .filter(|p| matches!(p.how, How::Order | How::Line))
            .map(|p| &lines[p.en]);
        let mut state = "manual";
        if let Some(en) = target {
            let new = en.edit_key.as_str();
            let taken: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM translators.proposals WHERE edit_key = $1 AND deleted_at IS NULL)
                     OR EXISTS (SELECT 1 FROM translators.approvals WHERE edit_key = $1)
                     OR EXISTS (SELECT 1 FROM translators.comments WHERE edit_key = $1 AND deleted_at IS NULL)",
            )
            .bind(new)
            .fetch_one(&mut **tx)
            .await?;
            let approved: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM translators.approvals WHERE edit_key = $1)")
                    .bind(&old)
                    .fetch_one(&mut **tx)
                    .await?;
            let shared = uses.get(new).copied().unwrap_or(0) > 1;
            if !taken && !(approved && shared) {
                let line = i32::try_from(en.line).unwrap_or(i32::MAX);
                // An entry the EN story shares with other lines: a proposal for this line only.
                sqlx::query(
                    "UPDATE translators.proposals SET edit_key = $2, story_id = $3, line = $4, part = $5,
                         context = context OR $6 WHERE edit_key = $1",
                )
                .bind(&old)
                .bind(new)
                .bind(story)
                .bind(line)
                .bind(en.part)
                .bind(shared)
                .execute(&mut **tx)
                .await?;
                for table in ["approvals", "comments"] {
                    sqlx::query(&format!(
                        "UPDATE translators.{table} SET edit_key = $2, story_id = $3, line = $4, part = $5 WHERE edit_key = $1"
                    ))
                    .bind(&old)
                    .bind(new)
                    .bind(story)
                    .bind(line)
                    .bind(en.part)
                    .execute(&mut **tx)
                    .await?;
                }
                sqlx::query(
                    "UPDATE translators.agreements SET edit_key = $2,
                         story_id = CASE WHEN story_id IS NULL THEN NULL ELSE $3 END,
                         line = CASE WHEN line IS NULL THEN NULL ELSE $4 END,
                         part = CASE WHEN part IS NULL THEN NULL ELSE $5 END
                     WHERE edit_key = $1",
                )
                .bind(&old)
                .bind(new)
                .bind(story)
                .bind(line)
                .bind(en.part)
                .execute(&mut **tx)
                .await?;
                sqlx::query("UPDATE translators.approval_log SET edit_key = $2 WHERE edit_key = $1")
                    .bind(&old)
                    .bind(new)
                    .execute(&mut **tx)
                    .await?;
                state = "moved";
            }
        }
        if state == "moved" {
            done.moved += 1;
        } else {
            done.manual += 1;
        }
        sqlx::query(
            "INSERT INTO translators.cn_transfers (old_key, story_id, state, new_key, line, part, cn)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (old_key) DO UPDATE SET state = EXCLUDED.state, new_key = EXCLUDED.new_key,
                 line = EXCLUDED.line, part = EXCLUDED.part,
                 cn = COALESCE(EXCLUDED.cn, translators.cn_transfers.cn), at = NOW()",
        )
        .bind(&old)
        .bind(story)
        .bind(state)
        .bind(target.filter(|_| state == "moved").map(|l| l.edit_key.clone()))
        .bind(target.map(|l| i32::try_from(l.line).unwrap_or(i32::MAX)))
        .bind(target.map(|l| l.part))
        .bind(cn_text)
        .execute(&mut **tx)
        .await?;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cn_only_entry_names_its_place() {
        let k = key("act55side_level_act55side_01_beg", 13, 0);
        assert_eq!(k, "cn:act55side_level_act55side_01_beg#13.0");
        assert_eq!(parse_key(&k), Some(("act55side_level_act55side_01_beg", 13, 0)));
        assert_eq!(parse_key("Hello."), None);
    }
}
