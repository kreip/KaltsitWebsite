//! Who says a text, by gender: its story lines' speakers (operators, by
//! `import/genders.rs`) and the operators whose voice line it is. One text
//! both a woman and a man say has one translation in the game's dictionary,
//! so a gendered one needs a separate translation for each (`mixedGender`).

use std::collections::HashSet;

use sqlx::PgPool;

use super::forms::Gender;
use crate::translators::dto::TranslatorLine;

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

/// Mark the lines whose entry operators of both genders say (`TranslatorLine::mixed_gender`).
pub async fn fill(
    pool: &PgPool,
    server: &str,
    lines: &mut [TranslatorLine],
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
    for l in lines {
        l.mixed_gender = mixed.contains(&l.edit_key);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
