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
    /// The Russian stem seen for it in the pack's text where the tag is kept
    /// (`умеренн-`, `урон искусств-`), the most frequent; null when no text keeps the tag.
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

/// Every English word or phrase in a colour tag of `texts` (English, the
/// pack's Russian), with the Russian stem the pack gives it where its text
/// keeps the same tag (the n-th span of that style); numbers and
/// substitutions are left out (the plugin colours those itself). The most
/// used first.
pub fn tag_words(texts: &[(String, Option<String>)]) -> Vec<TagWord> {
    #[derive(Default)]
    struct Acc {
        lines: i32,
        styles: HashMap<String, i32>,
        ru: HashMap<String, i32>,
    }
    let mut by_en: HashMap<String, Acc> = HashMap::new();
    for (en, ru) in texts {
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
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)));
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
}
