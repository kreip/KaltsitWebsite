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

/// The singular endings of a word by case — nominative, genitive, dative,
/// accusative, instrumental, prepositional — each with the endings accepted
/// for it, the usual one first.
pub type Cases = [&'static [&'static str]; 6];

const CASES_IA: Cases = [&["я"], &["и"], &["и"], &["ю"], &["ей", "ею"], &["и"]];
const CASES_YA: Cases = [&["я"], &["и"], &["е"], &["ю"], &["ей", "ею"], &["е"]];
const CASES_A: Cases = [&["а"], &["ы"], &["е"], &["у"], &["ой", "ою"], &["е"]];
/// After г к х: `и`, not `ы`.
const CASES_A_VELAR: Cases = [&["а"], &["и"], &["е"], &["у"], &["ой", "ою"], &["е"]];
/// After ж ш ч щ: `и`, and `ей` as well as `ой`.
const CASES_A_HUSHING: Cases = [
    &["а"],
    &["и"],
    &["е"],
    &["у"],
    &["ей", "ею", "ой", "ою"],
    &["е"],
];
/// A man's name or a noun in a consonant; the accusative as a person's (= genitive).
const CASES_CONSONANT: Cases = [&[""], &["а"], &["у"], &["а"], &["ом"], &["е"]];
const CASES_CONSONANT_HUSHING: Cases = [&[""], &["а"], &["у"], &["а"], &["ем", "ом"], &["е"]];
const CASES_YI: Cases = [&["ый"], &["ого"], &["ому"], &["ого", "ый"], &["ым"], &["ом"]];
const CASES_OE: Cases = [&["ое"], &["ого"], &["ому"], &["ое"], &["ым"], &["ом"]];
const CASES_AYA: Cases = [&["ая"], &["ой"], &["ой"], &["ую"], &["ой", "ою"], &["ой"]];
const CASES_AYA_HUSHING: Cases = [
    &["ая"],
    &["ей", "ой"],
    &["ей", "ой"],
    &["ую"],
    &["ей", "ею", "ой"],
    &["ей", "ой"],
];

/// A word's stem (what stays, lower case) and its endings by case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paradigm {
    pub stem: String,
    pub cases: &'static Cases,
}

impl Paradigm {
    /// Every ending of the word, the nominative's first.
    pub fn endings(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for e in self.cases.iter().flat_map(|c| c.iter().copied()) {
            if !out.contains(&e) {
                out.push(e);
            }
        }
        out
    }

    pub fn nominative(&self) -> &'static str {
        self.cases[0][0]
    }

    /// The cases `word` (lower case, `ё` = `е`) may stand in; empty when it is not a form of this word.
    pub fn cases_of(&self, word: &str) -> Vec<usize> {
        word.strip_prefix(self.stem.as_str())
            .map(|e| (0..6).filter(|&i| self.cases[i].contains(&e)).collect())
            .unwrap_or_default()
    }
}

/// The singular case forms of a word (lower case, `ё` = `е`) by simple
/// rules. Names and nouns in `-ия` (Амия: Амии, Амию, Амией), `-а`
/// (Маргарита: Маргариты, Маргарите, Маргариту, Маргаритой), `-я` (Зоя: Зои,
/// Зое, Зою, Зоей), a consonant (Рэндолл: Рэндолла, Рэндоллу, Рэндоллом,
/// Рэндолле); adjectives in `-ый`, `-ое`, `-ая`. None for any other word
/// (`-ь` may not decline at all — Чэнь; `-о`, `-и`, `-ий`…).
pub fn paradigm(word: &str) -> Option<Paradigm> {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    if n < 2 || !chars.iter().all(|c| c.is_alphabetic()) {
        return None;
    }
    let keep = |k: usize| chars[..n - k].iter().collect::<String>();
    let at = |k: usize| chars[n - 1 - k];
    let hushing = |c: char| "жшчщ".contains(c);
    let make = |k: usize, cases: &'static Cases| {
        Some(Paradigm {
            stem: keep(k),
            cases,
        })
    };
    let last = at(0);
    let end2: String = chars[n - 2..].iter().collect();
    if n >= 4 {
        match end2.as_str() {
            "ый" => return make(2, &CASES_YI),
            "ое" => return make(2, &CASES_OE),
            "ая" if hushing(at(2)) => return make(2, &CASES_AYA_HUSHING),
            "ая" => return make(2, &CASES_AYA),
            _ => {}
        }
    }
    match last {
        'я' if at(1) == 'и' => make(1, &CASES_IA),
        'я' => make(1, &CASES_YA),
        'а' if hushing(at(1)) => make(1, &CASES_A_HUSHING),
        'а' if "гкх".contains(at(1)) => make(1, &CASES_A_VELAR),
        'а' => make(1, &CASES_A),
        c if is_consonant(c) && (hushing(c) || c == 'ц') => make(0, &CASES_CONSONANT_HUSHING),
        c if is_consonant(c) => make(0, &CASES_CONSONANT),
        _ => None,
    }
}

/// Whether `word` is a case form of `form` by `paradigm` (both lower case, `ё` = `е`).
pub fn inflects(word: &str, form: &str) -> bool {
    paradigm(form).is_some_and(|p| !p.cases_of(word).is_empty())
}

/// A word that keeps one form in every case: no case rules, and it ends in a
/// vowel other than `а` / `я` (`-о`, `-е`, `-и`, `-у`, `-ю`, `-э`, `-ы`).
pub fn indeclinable(word: &str) -> bool {
    paradigm(word).is_none() && word.chars().next_back().is_some_and(|c| "оеиуюэы".contains(c))
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
