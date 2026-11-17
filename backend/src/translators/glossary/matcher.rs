//! Finding glossary terms in an English line, and whether the Russian line
//! carries the glossary's form.
//!
//! Rules (translators/README.md, "XUnity dictionary and plugin format"):
//! names, classes and races match only as capitalised in the glossary, the
//! "various terms" sheet in any case; longer terms win over shorter ones; the
//! Russian is compared by word stem (one ending cut, stem at least 3 letters,
//! `ё` = `е`, quotes and `ООО` ignored), a multi-word term word by word in a
//! row, an indeclinable name by prefix.

use std::collections::HashMap;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

use super::parse::Term;

/// Endings cut from a Russian word, longest first.
const ENDINGS: &[&str] = &[
    "ами", "ями", "ого", "его", "ому", "ему", "ыми", "ими", "ой", "ей", "ом", "ем", "ам", "ям",
    "ах", "ях", "ов", "ев", "ий", "ый", "ая", "яя", "ое", "ее", "ую", "юю", "ы", "и", "а", "я",
    "у", "ю", "е", "о", "ь", "й",
];

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .replace('ё', "е")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The byte spans of a text's words (runs of letters and digits), in order.
pub fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, text.len()));
    }
    out
}

/// The stem the kit compares by.
pub fn stem(word: &str) -> String {
    for e in ENDINGS {
        if let Some(base) = word.strip_suffix(e)
            && base.chars().count() >= 3
        {
            return base.to_owned();
        }
    }
    word.to_owned()
}

/// Whether `text` holds `form`, word by word in a row: by stem, or as one of
/// the word's case forms (`paradigm`).
pub fn has_form(text: &str, form: &str) -> bool {
    let cleaned = form.replace(['"', '«', '»', '„', '“', '”'], " ");
    let forms: Vec<String> = words(&cleaned)
        .into_iter()
        .filter(|w| w != "ооо")
        .collect();
    if forms.is_empty() {
        return false;
    }
    let stems: Vec<String> = forms.iter().map(|w| stem(w)).collect();
    let text_words = words(text);
    text_words.windows(forms.len()).any(|win| {
        win.iter()
            .zip(forms.iter().zip(&stems))
            .all(|(w, (f, s))| w.starts_with(s.as_str()) || inflects(w, f))
    })
}

pub fn is_consonant(c: char) -> bool {
    "бвгджзклмнпрстфхцчшщ".contains(c)
}

const ENDS_IA: &[&str] = &["я", "и", "ю", "ей", "ею"];
const ENDS_YA: &[&str] = &["я", "и", "е", "ю", "ей", "ею"];
const ENDS_A: &[&str] = &["а", "ы", "е", "у", "ой", "ою"];
/// After г к х ж ш ч щ: `и`, not `ы`; after a hushing letter `ей` too.
const ENDS_A_SOFT: &[&str] = &["а", "и", "е", "у", "ой", "ою", "ей", "ею"];
const ENDS_CONSONANT: &[&str] = &["", "а", "у", "ом", "е"];
const ENDS_CONSONANT_HUSHING: &[&str] = &["", "а", "у", "ом", "ем", "е"];
const ENDS_YI: &[&str] = &["ый", "ого", "ому", "ым", "ом"];
const ENDS_OE: &[&str] = &["ое", "ого", "ому", "ым", "ом"];
const ENDS_AYA: &[&str] = &["ая", "ой", "ую", "ою"];
const ENDS_AYA_HUSHING: &[&str] = &["ая", "ей", "ую", "ею", "ой"];

/// The singular case forms of a glossary word (lower case, `ё` = `е`) by
/// simple rules: its stem (what stays) and the endings, the nominative's
/// first. Names and nouns in `-ия` (Амия: Амии, Амию, Амией), `-а`
/// (Маргарита: Маргариты, Маргарите, Маргариту, Маргаритой), `-я` (Зоя: Зои,
/// Зое, Зою, Зоей), a consonant (Рэндолл: Рэндолла, Рэндоллу, Рэндоллом,
/// Рэндолле); adjectives in `-ый`, `-ое`, `-ая`. None for any other word
/// (`-ь` may not decline at all — Чэнь; `-о`, `-и`, `-ий`…).
pub fn paradigm(word: &str) -> Option<(String, &'static [&'static str])> {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    if n < 2 || !chars.iter().all(|c| c.is_alphabetic()) {
        return None;
    }
    let keep = |k: usize| chars[..n - k].iter().collect::<String>();
    let at = |k: usize| chars[n - 1 - k];
    let hushing = |c: char| "жшчщц".contains(c);
    let velar = |c: char| "гкхжшчщ".contains(c);
    let last = at(0);
    let end2: String = chars[n - 2..].iter().collect();
    if n >= 4 {
        match end2.as_str() {
            "ый" => return Some((keep(2), ENDS_YI)),
            "ое" => return Some((keep(2), ENDS_OE)),
            "ая" if hushing(at(2)) => return Some((keep(2), ENDS_AYA_HUSHING)),
            "ая" => return Some((keep(2), ENDS_AYA)),
            _ => {}
        }
    }
    match last {
        'я' if at(1) == 'и' => Some((keep(1), ENDS_IA)),
        'я' => Some((keep(1), ENDS_YA)),
        'а' if velar(at(1)) => Some((keep(1), ENDS_A_SOFT)),
        'а' => Some((keep(1), ENDS_A)),
        c if is_consonant(c) && hushing(c) => Some((keep(0), ENDS_CONSONANT_HUSHING)),
        c if is_consonant(c) => Some((keep(0), ENDS_CONSONANT)),
        _ => None,
    }
}

/// Whether `word` is a case form of `form` by `paradigm` (both lower case, `ё` = `е`).
pub fn inflects(word: &str, form: &str) -> bool {
    paradigm(form).is_some_and(|(stem, ends)| {
        word.strip_prefix(stem.as_str())
            .is_some_and(|e| ends.contains(&e))
    })
}

/// Whether `text` holds any of a term's Russian forms (variants split by ` / `,
/// plus the plural; a bracket such as `(Страж)` is not looked for).
pub fn term_found(text: &str, term: &Term) -> bool {
    term.ru
        .split(" / ")
        .chain(term.ru_plural.as_deref())
        .any(|f| has_form(text, &super::parse::split_bracket(f).0))
}

/// The glossary indexed for finding its keys in English text.
pub struct Finder {
    exact: Option<AhoCorasick>,
    exact_terms: Vec<Vec<usize>>,
    folded: Option<AhoCorasick>,
    folded_terms: Vec<Vec<usize>>,
}

/// One term key found in a text: the byte range and the terms (rows) it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub start: usize,
    pub end: usize,
    pub terms: Vec<usize>,
}

fn build(keys: Vec<(String, Vec<usize>)>, fold: bool) -> (Option<AhoCorasick>, Vec<Vec<usize>>) {
    if keys.is_empty() {
        return (None, Vec::new());
    }
    let (patterns, terms): (Vec<String>, Vec<Vec<usize>>) = keys.into_iter().unzip();
    let ac = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(fold)
        .build(&patterns)
        .ok();
    (ac, terms)
}

fn is_word_char(c: Option<char>) -> bool {
    c.is_some_and(char::is_alphanumeric)
}

impl Finder {
    pub fn new(terms: &[Term]) -> Self {
        let mut exact: HashMap<String, Vec<usize>> = HashMap::new();
        let mut folded: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, t) in terms.iter().enumerate() {
            if t.case_sensitive {
                exact.entry(t.key.clone()).or_default().push(i);
            } else {
                folded.entry(t.key.to_lowercase()).or_default().push(i);
            }
        }
        let (exact, exact_terms) = build(exact.into_iter().collect(), false);
        let (folded, folded_terms) = build(folded.into_iter().collect(), true);
        Self {
            exact,
            exact_terms,
            folded,
            folded_terms,
        }
    }

    /// Every term key in `text` on word boundaries; overlaps go to the longer key.
    pub fn find(&self, text: &str) -> Vec<Found> {
        let mut all = Vec::new();
        for (ac, terms) in [
            (&self.exact, &self.exact_terms),
            (&self.folded, &self.folded_terms),
        ] {
            let Some(ac) = ac else { continue };
            for m in ac.find_iter(text) {
                let before = text[..m.start()].chars().next_back();
                let after = text[m.end()..].chars().next();
                if is_word_char(before) || is_word_char(after) {
                    continue;
                }
                all.push(Found {
                    start: m.start(),
                    end: m.end(),
                    terms: terms[m.pattern().as_usize()].clone(),
                });
            }
        }
        all.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then((b.end - b.start).cmp(&(a.end - a.start)))
        });
        let mut out: Vec<Found> = Vec::new();
        for f in all {
            if out.last().is_some_and(|l| f.start < l.end) {
                continue;
            }
            out.push(f);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(key: &str, ru: &str, cs: bool) -> Term {
        Term {
            key: key.into(),
            en: key.into(),
            ru: ru.into(),
            case_sensitive: cs,
            ..Term::default()
        }
    }

    #[test]
    fn russian_forms_match_by_stem() {
        assert!(has_form("Мы вернулись на Родос Айленд.", "Родос Айленд"));
        assert!(has_form("Без Амии ничего не выйдет.", "Амия"));
        assert!(has_form("Спросите Маунтена.", "Маунтен"));
        assert!(has_form(
            "Сотрудники фармацевтической компании «Родос Айленд»",
            "Фармацевтическая компания \"Родос Айленд\""
        ));
        assert!(has_form("Ёжик", "ежик"));
        assert!(!has_form("Айленд Родос", "Родос Айленд"));
        assert!(!has_form("Ничего нет", "Амия"));
    }

    #[test]
    fn simple_case_forms_count() {
        for text in ["У Зои царапина.", "Скажи Зое.", "Вижу Зою.", "С Зоей.", "Зоя тут."] {
            assert!(has_form(text, "Зоя"), "{text}");
        }
        assert!(!has_form("У Зоуи царапина.", "Зоя"));
        assert!(has_form("Спроси Амию.", "Амия"));
        assert!(has_form("С Амией.", "Амия"));
        assert!(has_form("Письмо Рэндоллу.", "Рэндолл"));
        assert!(has_form("Он с Рэндоллом.", "Рэндолл"));
        assert!(has_form("С Маргаритой.", "Маргарита"));
        assert!(has_form("У Ольги.", "Ольга"));
        assert!(!has_form("Чени нет.", "Чэнь"));
        assert_eq!(paradigm("чэнь"), None);
        assert!(inflects("фармацевтической", "фармацевтическая"));
        assert!(!inflects("фармацевтическаой", "фармацевтическая"));
    }

    #[test]
    fn plural_counts_for_races() {
        let mut t = term("Sarkaz", "Сарказ", true);
        t.ru_plural = Some("Саркази".into());
        assert!(term_found("Эти саркази опасны.", &t));
    }

    #[test]
    fn the_russian_bracket_is_not_looked_for() {
        let t = term("Amiya", "Амия (Страж)", true);
        assert!(term_found("Амия кивнула.", &t));
    }

    #[test]
    fn longer_terms_win_and_case_rules_hold() {
        let terms = vec![
            term("Rhodes Island", "Родос Айленд", true),
            term("Rhodes", "Родос", true),
            term("Guard", "Страж", true),
            term("originium", "ориджиниум", false),
        ];
        let f = Finder::new(&terms);
        let found = f.find("Rhodes Island's guard found Originium.");
        let keys: Vec<&str> = found
            .iter()
            .map(|m| terms[m.terms[0]].key.as_str())
            .collect();
        assert_eq!(keys, ["Rhodes Island", "originium"]);
        assert!(f.find("Guardian").is_empty());
    }
}
