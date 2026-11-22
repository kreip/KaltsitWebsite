//! Who says a text, by gender: its story lines' speakers (operators, by
//! `import/genders.rs`) and the operators whose voice line it is. One text
//! both a woman and a man say has one translation in the game's dictionary,
//! so a gendered one needs a separate translation for each (`mixedGender`): the
//! line lists them all. Also the operator whose text a text outside the story is.

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use super::forms::{Gender, speaker_gender_in};
use crate::translators::dto::{LineSpeaker, TranslatorLine};
use crate::translators::edits::dto::KeyEdits;

/// Who says one entry, as far as the game's files tell.
#[derive(Debug, Clone, Copy, Default)]
pub struct Said {
    pub woman: bool,
    pub man: bool,
    /// A story line of it is said by someone of no known gender.
    pub unknown: bool,
    /// It is an operator's voice line of a known gender: theirs whoever else says it.
    pub voiced: bool,
}

impl Said {
    /// Both a woman and a man say it.
    pub fn mixed(self) -> bool {
        self.woman && self.man
    }

    /// The gender to check it against: the only one it is said with, when that is sure.
    pub fn gender(self) -> Option<Gender> {
        if self.mixed() || (self.unknown && !self.voiced) {
            None
        } else if self.woman {
            Some('f')
        } else if self.man {
            Some('m')
        } else {
            None
        }
    }
}

/// Mark the lines whose entry operators of both genders say (`TranslatorLine::mixed_gender`)
/// and list those operators where a Russian of the line has a gender (`mixed_speakers`): a plain
/// «Huh?» has 400 speakers and nothing to translate apart.
pub async fn fill(
    pool: &PgPool,
    server: &str,
    lines: &mut [TranslatorLine],
    edits: &[KeyEdits],
) -> Result<(), sqlx::Error> {
    let keys: Vec<String> = lines
        .iter()
        .map(|l| l.edit_key.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if keys.is_empty() {
        return Ok(());
    }
    let mixed: HashSet<String> = sqlx::query_scalar(
        "SELECT said.edit_key FROM (
             SELECT edit_key, speaker_gender AS gender FROM translators.story_lines
             WHERE server = $1 AND edit_key = ANY($2) AND speaker_gender IS NOT NULL
             UNION ALL
             SELECT p.edit_key, g.gender FROM translators.text_places p
             JOIN translators.char_genders g ON g.char_id = p.object_id
             WHERE p.server = $1 AND p.edit_key = ANY($2) AND p.field = 'voice.text'
         ) said GROUP BY 1 HAVING COUNT(DISTINCT said.gender) = 2",
    )
    .bind(server)
    .bind(&keys)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    let mut gendered: HashSet<String> = HashSet::new();
    for l in lines.iter_mut() {
        l.mixed_gender = mixed.contains(&l.edit_key);
        if l.mixed_gender && has_gender(l, edits) {
            gendered.insert(l.edit_key.clone());
        }
    }
    if gendered.is_empty() {
        return Ok(());
    }
    let who = who_says(pool, server, &gendered.into_iter().collect::<Vec<_>>()).await?;
    for l in lines {
        if let Some(list) = who.get(&l.edit_key) {
            l.mixed_speakers.clone_from(list);
        }
    }
    Ok(())
}

/// Whether any Russian of the line gives its speaker a gender.
fn has_gender(line: &TranslatorLine, edits: &[KeyEdits]) -> bool {
    let gendered = |ru: &str| speaker_gender_in(ru).is_some();
    [&line.ru_game, &line.sova, &line.artem, &line.ai]
        .into_iter()
        .flatten()
        .any(|ru| gendered(ru))
        || edits
            .iter()
            .filter(|e| e.edit_key == line.edit_key)
            .any(|e| {
                e.approval.as_ref().is_some_and(|a| gendered(&a.text))
                    || e.proposals.iter().any(|p| gendered(&p.text))
            })
}

/// Every operator who says each entry, once: at their voice line, else at their first story line.
async fn who_says(
    pool: &PgPool,
    server: &str,
    keys: &[String],
) -> Result<HashMap<String, Vec<LineSpeaker>>, sqlx::Error> {
    let voices: Vec<(String, String, i16)> = sqlx::query_as(
        "SELECT DISTINCT ON (edit_key, object_id) edit_key, object_id, idx
         FROM translators.text_places
         WHERE server = $1 AND edit_key = ANY($2) AND field = 'voice.text'
         ORDER BY edit_key, object_id, ord",
    )
    .bind(server)
    .bind(keys)
    .fetch_all(pool)
    .await?;
    let ids: Vec<String> = voices.iter().map(|(_, id, _)| id.clone()).collect();
    let known = operators(pool, server, &ids).await?;
    let mut out: HashMap<String, Vec<LineSpeaker>> = HashMap::new();
    for (key, id, idx) in voices {
        if let Some(operator) = known.get(&id) {
            out.entry(key).or_default().push(LineSpeaker {
                voice: Some(idx),
                ..operator.clone()
            });
        }
    }
    #[allow(clippy::type_complexity)]
    let story: Vec<(String, String, Option<String>, String, String, i32)> = sqlx::query_as(
        "SELECT DISTINCT ON (edit_key, speaker_en) edit_key, speaker_en, speaker_ru, speaker_gender, story_id, ord
         FROM translators.story_lines
         WHERE server = $1 AND edit_key = ANY($2) AND speaker_gender IS NOT NULL
         ORDER BY edit_key, speaker_en, story_id, ord",
    )
    .bind(server)
    .bind(keys)
    .fetch_all(pool)
    .await?;
    for (key, name_en, name_ru, gender, story_id, ord) in story {
        let list = out.entry(key).or_default();
        if list.iter().all(|s| s.name_en != name_en) {
            list.push(LineSpeaker {
                name_en,
                name_ru,
                gender: Some(gender),
                story_id: Some(story_id),
                ord: Some(ord),
                ..LineSpeaker::default()
            });
        }
    }
    Ok(out)
}

/// Operators by id: the name (in the game's Russian too) and the gender their file states.
async fn operators(
    pool: &PgPool,
    server: &str,
    ids: &[String],
) -> Result<HashMap<String, LineSpeaker>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT DISTINCT ON (p.object_id) p.object_id, p.object_en, t.ru, g.gender
         FROM translators.text_places p
         LEFT JOIN translators.texts t
             ON p.section = 'operators' AND p.field = 'name' AND t.server = p.server AND t.edit_key = p.edit_key
         LEFT JOIN translators.char_genders g ON g.char_id = p.object_id
         WHERE p.server = $1 AND p.section IN ('operators', 'archives') AND p.object_id = ANY($2)
         ORDER BY p.object_id, (p.section = 'operators' AND p.field = 'name') DESC, p.ord",
    )
    .bind(server)
    .bind(ids)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name_en, name_ru, gender)| {
            let operator = LineSpeaker {
                name_en,
                name_ru,
                gender,
                char_id: Some(id.clone()),
                ..LineSpeaker::default()
            };
            (id, operator)
        })
        .collect())
}

/// Name the operator of each text outside the story that is one's (`TranslatorLine::owner`):
/// `ids` gives each line's, if it has one.
pub async fn fill_owners(
    pool: &PgPool,
    server: &str,
    lines: &mut [TranslatorLine],
    ids: &[Option<&str>],
) -> Result<(), sqlx::Error> {
    let wanted: Vec<String> = ids
        .iter()
        .flatten()
        .map(|id| (*id).to_owned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let known = operators(pool, server, &wanted).await?;
    for (line, id) in lines.iter_mut().zip(ids) {
        if line.kind == "text" {
            line.owner = id.and_then(|id| known.get(id).cloned());
        }
    }
    Ok(())
}

/// The id, when a place of the game data is an operator's (no summon, no device).
pub fn operator_at<'a>(section: &str, object_id: &'a str) -> Option<&'a str> {
    ((section == super::gamedata::OPERATORS || section == super::gamedata::ARCHIVES)
        && object_id.starts_with("char_"))
    .then_some(object_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_is_an_operators_in_their_card_and_archives() {
        let warmy = "char_4081_warmy";
        assert_eq!(operator_at("archives", warmy), Some(warmy));
        assert_eq!(operator_at("operators", "token_10000_silent_healrb"), None);
        assert_eq!(operator_at("items", warmy), None);
    }

    #[test]
    fn whose_gender_a_text_is_checked_against() {
        let woman = Said {
            woman: true,
            ..Said::default()
        };
        assert_eq!(woman.gender(), Some('f'));
        // Franka's voice line and Logos's line of a story: both, so neither.
        let both = Said { man: true, ..woman };
        assert!(both.mixed());
        assert_eq!(both.gender(), None);
        // A nameless one says it too: only a voice line is still sure whose it is.
        let shared = Said {
            unknown: true,
            ..woman
        };
        assert_eq!(shared.gender(), None);
        let voice = Said {
            voiced: true,
            ..shared
        };
        assert_eq!(voice.gender(), Some('f'));
    }
}
