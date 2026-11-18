//! The words of the game's colour tags (`<@ba.vup>moderately</>`, `<@ba.kw>Arts damage</>`)
//! and the Russian the pack writes for them where its text keeps the tag: for the admin
//! panel's «Теги», the list the RichTextFix plugin's word rules are made from.

use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;
use utoipa::ToSchema;

use crate::translators::glossary::matcher::stem;

/// One English word or phrase found in a colour tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TagWord {
    /// As the English writes it, lower case.
    pub en: String,
    /// The styles it stands in (`ba.vup`, `ba.kw`), the most used first.
    pub styles: Vec<String>,
    /// Texts whose English has it in a colour tag.
    pub lines: i32,
    /// The Russian stem the pack writes for it (`умеренн-`, `урон искусств`): where
    /// the tag is kept, the most frequent; else the stems that stand with it; null
    /// when none is sure.
    pub ru: Option<String>,
    /// Texts that gave that stem.
    pub ru_lines: i32,
    /// The plugin's rule for it; null — not known yet (the plugin's rules are not read).
    pub plugin_rule: Option<String>,
}

/// The colour spans of a text: (style, inner text) in order; spans with
/// nested tags are skipped.
fn spans(text: &str) -> Vec<(&str, &str)> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re =
        RE.get_or_init(|| regex::Regex::new(r"<@([A-Za-z0-9_.]+)>([^<]*)</>").expect("tag regex"));
    re.captures_iter(text)
        .filter_map(|c| Some((c.get(1)?.as_str(), c.get(2)?.as_str())))
        .collect()
}

/// A Russian phrase as stems, each cut word marked with `-`: «урона Искусств» → «урон искусств-».
fn ru_stem(phrase: &str) -> String {
    phrase
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let lower = w.to_lowercase().replace('ё', "е");
            let st = stem(&lower);
            if st == lower { st } else { format!("{st}-") }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A share of a phrase's texts a Russian stem must stand in to be its word,
/// and a share of all the stem's texts that must be the phrase's.
const MIN_SHARE: f64 = 0.4;
const MIN_PRECISION: f64 = 0.2;

/// The Russian words of a text as (stem, shown form): «умеренно» → («умеренн», «умеренн-»).
fn ru_words(text: &str) -> Vec<(String, String)> {
    static TAG: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let tag = TAG.get_or_init(|| regex::Regex::new(r"<[^>]*>|\{[^}]*\}").expect("tag regex"));
    tag.replace_all(text, " ")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3 && w.chars().all(char::is_alphabetic))
        .map(|w| {
            let lower = w.to_lowercase().replace('ё', "е");
            let st = stem(&lower);
            let shown = if st == lower {
                st.clone()
            } else {
                format!("{st}-")
            };
            (st, shown)
        })
        .collect()
}

/// Every English word or phrase in a colour tag of `texts` (English, the
/// pack's Russian), with the Russian stem the pack gives it: where its text
/// keeps the same tag, the n-th span of that style; else (the pack drops the
/// tags) the stems standing in at least `MIN_SHARE` of the phrase's texts and
/// rarely elsewhere (`MIN_PRECISION`), as many as the phrase has words, in the
/// order the Russian writes them. Numbers and substitutions are left out (the
/// plugin colours those itself). The most used first.
pub fn tag_words(texts: &[(String, Option<String>)]) -> Vec<TagWord> {
    #[derive(Default)]
    struct Acc {
        lines: i32,
        styles: HashMap<String, i32>,
        ru: HashMap<String, i32>,
        /// The texts it stands in (indices of `texts`).
        texts: Vec<usize>,
    }
    let mut by_en: HashMap<String, Acc> = HashMap::new();
    // Each text's Russian stems once, and in how many texts each stem stands.
    let words: Vec<Vec<(String, String)>> = texts
        .iter()
        .map(|(_, ru)| {
            let mut seen = std::collections::HashSet::new();
            ru.as_deref()
                .map(ru_words)
                .unwrap_or_default()
                .into_iter()
                .filter(|(st, _)| seen.insert(st.clone()))
                .collect()
        })
        .collect();
    let mut df: HashMap<&str, u32> = HashMap::new();
    for ws in &words {
        for (st, _) in ws {
            *df.entry(st.as_str()).or_default() += 1;
        }
    }
    for (i, (en, ru)) in texts.iter().enumerate() {
        let en_spans = spans(en);
        if en_spans.is_empty() {
            continue;
        }
        let ru_spans = ru.as_deref().map(spans).unwrap_or_default();
        let mut seen: Vec<String> = Vec::new();
        let mut nth: HashMap<&str, usize> = HashMap::new();
        for (style, inner) in &en_spans {
            let k = nth.entry(style).or_default();
            let at = *k;
            *k += 1;
            let phrase = inner.trim().to_lowercase();
            if !phrase.chars().any(char::is_alphabetic) || phrase.contains('{') {
                continue;
            }
            let acc = by_en.entry(phrase.clone()).or_default();
            if !seen.contains(&phrase) {
                acc.lines += 1;
                acc.texts.push(i);
                seen.push(phrase.clone());
            }
            *acc.styles.entry((*style).to_owned()).or_default() += 1;
            // The same style's n-th span in the Russian.
            if let Some((_, ru_inner)) = ru_spans.iter().filter(|(s, _)| s == style).nth(at) {
                let st = ru_stem(ru_inner);
                if st.chars().any(char::is_alphabetic) {
                    *acc.ru.entry(st).or_default() += 1;
                }
            }
        }
    }
    let mut out: Vec<TagWord> = by_en
        .into_iter()
        .map(|(en, acc)| {
            let mut styles: Vec<(String, i32)> = acc.styles.into_iter().collect();
            styles.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let best = acc
                .ru
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                .or_else(|| cooccurring(&en, &acc.texts, &words, &df));
            TagWord {
                en,
                styles: styles.into_iter().map(|s| s.0).collect(),
                lines: acc.lines,
                ru_lines: best.as_ref().map_or(0, |b| b.1),
                ru: best.map(|b| b.0),
                plugin_rule: None,
            }
        })
        .collect();
    out.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.en.cmp(&b.en)));
    out
}

/// The pack drops the tags: the Russian stems that stand with the phrase
/// (see `tag_words`), joined in the Russian's order, and in how many of its
/// texts they all stand.
fn cooccurring(
    en: &str,
    at: &[usize],
    words: &[Vec<(String, String)>],
    df: &HashMap<&str, u32>,
) -> Option<(String, i32)> {
    let n = at.len();
    if n < 2 {
        return None;
    }
    let mut count: HashMap<&str, u32> = HashMap::new();
    for &i in at {
        for (st, _) in &words[i] {
            *count.entry(st.as_str()).or_default() += 1;
        }
    }
    let want = en.split_whitespace().count().clamp(1, 3);
    let mut picked: Vec<(&str, f64)> = count
        .into_iter()
        .filter_map(|(st, c)| {
            let share = f64::from(c) / n as f64;
            let precision = f64::from(c) / f64::from(*df.get(st)?);
            (share >= MIN_SHARE && precision >= MIN_PRECISION).then_some((st, share * precision))
        })
        .collect();
    picked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    picked.truncate(want);
    if picked.is_empty() {
        return None;
    }
    // In the order of the first text holding them all (else of the first text).
    let holds = |i: usize| {
        picked
            .iter()
            .all(|(st, _)| words[i].iter().any(|(w, _)| w == st))
    };
    let first = at.iter().copied().find(|&i| holds(i)).unwrap_or(at[0]);
    let shown: Vec<&str> = words[first]
        .iter()
        .filter(|(w, _)| picked.iter().any(|(st, _)| st == w))
        .map(|(_, s)| s.as_str())
        .collect();
    if shown.is_empty() {
        return None;
    }
    let all = i32::try_from(at.iter().filter(|&&i| holds(i)).count()).unwrap_or(i32::MAX);
    Some((shown.join(" "), all))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_in_colour_tags_with_the_pack_s_stem() {
        let texts = vec![
            (
                "Deals <@ba.kw>Arts damage</> and <@ba.vup>moderately</> slows; <@ba.vup>+30%</> ATK".to_owned(),
                Some("Наносит <@ba.kw>урон Искусств</> и <@ba.vup>умеренно</> замедляет; <@ba.vup>+30%</> АТК".to_owned()),
            ),
            (
                "<@ba.vup>Moderately</> heals".to_owned(),
                Some("Умеренно лечит".to_owned()),
            ),
        ];
        let words = tag_words(&texts);
        assert_eq!(words[0].en, "moderately");
        assert_eq!(words[0].lines, 2);
        assert_eq!(words[0].ru.as_deref(), Some("умеренн-"));
        assert_eq!(words[0].ru_lines, 1);
        let arts = words.iter().find(|w| w.en == "arts damage").expect("arts");
        assert_eq!(arts.ru.as_deref(), Some("урон искусств"));
        assert_eq!(arts.styles, ["ba.kw"]);
        // Numbers are the plugin's own.
        assert!(words.iter().all(|w| w.en != "+30%"));
    }

    #[test]
    fn without_the_tags_the_stem_that_stands_with_the_word() {
        let texts = vec![
            (
                "Drags it <@ba.vup>moderately</>".to_owned(),
                Some("Умеренно притягивает цель".to_owned()),
            ),
            (
                "Pushes <@ba.vup>moderately</>".to_owned(),
                Some("Умеренно отталкивает цель".to_owned()),
            ),
            (
                "Deals <@ba.kw>Arts damage</>".to_owned(),
                Some("Наносит урон Искусств".to_owned()),
            ),
            (
                "Deals <@ba.kw>Arts damage</> twice".to_owned(),
                Some("Дважды наносит урон Искусств".to_owned()),
            ),
            (
                "Deals Physical damage".to_owned(),
                Some("Наносит физический урон цель".to_owned()),
            ),
            ("Strikes".to_owned(), Some("Наносит удар".to_owned())),
            (
                "Strikes twice".to_owned(),
                Some("Дважды наносит удар".to_owned()),
            ),
        ];
        let words = tag_words(&texts);
        let m = words
            .iter()
            .find(|w| w.en == "moderately")
            .expect("moderately");
        assert_eq!(m.ru.as_deref(), Some("умеренн-"));
        assert_eq!(m.ru_lines, 2);
        let arts = words.iter().find(|w| w.en == "arts damage").expect("arts");
        assert_eq!(arts.ru.as_deref(), Some("урон искусств"));
    }
}
