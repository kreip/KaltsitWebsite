//! A term's Russian in a line against the glossary's form, word by word, and
//! the glossary's form put into the line in the same case ("Рэндаллу" →
//! "Рэндоллу"): the "Replace with the glossary's form" button of the Glossary
//! tab and the "As in the game" column for a term of several words.
//!
//! The case is carried over by the glossary word's own case forms
//! (`matcher::paradigm`: names and nouns in -ия, -а, -я, a consonant; a few
//! adjective endings): the game word's ending is found among them and the
//! glossary's stem gets it, when the stems are the same word spelled
//! differently (a letter or two apart, ending alike). The game word in the
//! nominative gets the form itself. A word whose case cannot be told that
//! way ("Зои" for "Зоуи", "апреля" for "Эйприл") is left to a person: the
//! nominative is never put in for another case ("У Зоя царапина").

use super::matcher::{inflects, is_consonant, paradigm, stem, word_spans};

/// One word of the game's text where a term stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordHit {
    pub start: usize,
    pub end: usize,
    /// The word is the glossary's (by stem).
    pub ok: bool,
}

/// Where a term stands in a text: as many words as the glossary's form has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub words: Vec<WordHit>,
}

impl Hit {
    pub fn start(&self) -> usize {
        self.words.first().map_or(0, |w| w.start)
    }

    pub fn end(&self) -> usize {
        self.words.last().map_or(0, |w| w.end)
    }

    pub fn all_ok(&self) -> bool {
        self.words.iter().all(|w| w.ok)
    }

    /// The words as written, one space between them.
    pub fn spelling(&self, text: &str) -> String {
        self.words
            .iter()
            .map(|w| &text[w.start..w.end])
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The words' stems, one space between them: the same variant in every ending.
    pub fn key(&self, text: &str) -> String {
        self.words
            .iter()
            .map(|w| stem(&fold(&text[w.start..w.end])))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The positions of the words that are not the glossary's.
    pub fn wrong(&self) -> Vec<usize> {
        self.words
            .iter()
            .enumerate()
            .filter(|(_, w)| !w.ok)
            .map(|(i, _)| i)
            .collect()
    }
}

fn fold(w: &str) -> String {
    w.to_lowercase().replace('ё', "е")
}

/// The stems of a glossary form's words (quotes and `ООО` dropped, as `has_form`).
pub fn form_stems(form: &str) -> Vec<String> {
    form.replace(['"', '«', '»', '„', '“', '”'], " ")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(fold)
        .filter(|w| w != "ооо")
        .map(|w| stem(&w))
        .collect()
}

/// The words of a glossary form, lower case, `ё` = `е` (quotes and `ООО`
/// dropped, as `has_form`).
pub fn form_folded(form: &str) -> Vec<String> {
    form.replace(['"', '«', '»', '„', '“', '”'], " ")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(fold)
        .filter(|w| w != "ооо")
        .collect()
}

/// Whether a game word (lower case, `ё` = `е`) is the glossary's word: by
/// stem or as one of its case forms.
pub fn is_form_word(word: &str, form: &str) -> bool {
    super::matcher::stem_match(word, &stem(form)) || inflects(word, form)
}

/// Where a form of `forms.len()` words (`form_folded`) stands in `text`: the
/// run of as many words with the most words matching the form (by stem or a
/// case form), position by position (at least one); ties go to the run with
/// more capitalised words, then the earliest. None when no word of the form
/// is in the text.
pub fn best_window(text: &str, forms: &[String]) -> Option<Hit> {
    if forms.is_empty() {
        return None;
    }
    let spans = word_spans(text);
    let mut best: Option<(usize, usize, Hit)> = None;
    for win in spans.windows(forms.len()) {
        let words: Vec<WordHit> = win
            .iter()
            .zip(forms)
            .map(|(&(start, end), f)| WordHit {
                start,
                end,
                ok: is_form_word(&fold(&text[start..end]), f),
            })
            .collect();
        let ok = words.iter().filter(|w| w.ok).count();
        if ok == 0 {
            continue;
        }
        let caps = win
            .iter()
            .filter(|&&(s, _)| text[s..].chars().next().is_some_and(char::is_uppercase))
            .count();
        if best
            .as_ref()
            .is_none_or(|(o, c, _)| ok > *o || (ok == *o && caps > *c))
        {
            best = Some((ok, caps, Hit { words }));
        }
    }
    best.map(|(_, _, h)| h)
}

/// The first word of `text` whose stem is `root` or starts with it (the
/// variant rule of the "As in the game" column: a root of four letters or
/// more covers the longer stems of the same word).
pub fn root_word(text: &str, root: &str) -> Option<(usize, usize)> {
    word_spans(text).into_iter().find(|&(s, e)| {
        let st = stem(&fold(&text[s..e]));
        st == root || (root.chars().count() >= 4 && st.starts_with(root))
    })
}

/// `word` with the case of `like`: capitalised when `like` is, all capitals when `like` is.
fn with_case(word: &str, like: &str) -> String {
    let mut chars = like.chars();
    let first = chars.next();
    if first.is_some_and(char::is_uppercase) {
        if like.chars().count() > 1 && like.chars().all(|c| !c.is_lowercase()) {
            return word.to_uppercase();
        }
        let mut out = String::new();
        let mut w = word.chars();
        if let Some(c) = w.next() {
            out.extend(c.to_uppercase());
        }
        out.extend(w);
        return out;
    }
    if first.is_some_and(char::is_lowercase) {
        let mut out = String::new();
        let mut w = word.chars();
        if let Some(c) = w.next() {
            out.extend(c.to_lowercase());
        }
        out.extend(w);
        return out;
    }
    word.to_owned()
}

/// Letters to change, add or drop to turn `a` into `b`.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The same word spelled differently: a letter apart, two for long stems.
fn same_word(a: &str, b: &str) -> bool {
    let n = a.chars().count().min(b.chars().count());
    distance(a, b) <= if n >= 6 { 2 } else { 1 }
}

/// Two stems are alike when they end alike: in the same letter, or both in a consonant.
fn end_alike(a: &str, b: &str) -> bool {
    match (a.chars().next_back(), b.chars().next_back()) {
        (Some(x), Some(y)) => x == y || (is_consonant(x) && is_consonant(y)),
        _ => false,
    }
}

/// The glossary's `form` word in the case of the game's `word`, or None
/// when the case cannot be carried over for sure.
pub fn fix_word(word: &str, form: &str) -> Option<String> {
    let lower = fold(word);
    let form_lower = fold(form);
    if let Some(p) = paradigm(&form_lower) {
        let form_stem = p.stem.clone();
        let kept: String = form.chars().take(form_stem.chars().count()).collect();
        let nominative = p.nominative();
        let mut ends = p.endings();
        ends.sort_by_key(|e| std::cmp::Reverse(e.chars().count()));
        for e in ends {
            let Some(base) = lower.strip_suffix(e) else {
                continue;
            };
            if base.is_empty() {
                continue;
            }
            // The game word in the nominative: the form itself, whatever its spelling.
            let sure = base == form_stem
                || (same_word(base, &form_stem) && (end_alike(base, &form_stem) || e == nominative));
            if sure {
                return Some(with_case(&format!("{kept}{e}"), word));
            }
        }
        return None;
    }
    // No case forms known: only when the game word ends as the form does, and
    // neither word may be another word's oblique case ("Зои" / "Зое": Zoey,
    // or Зоя's genitive / dative) — the case is a guess then.
    if may_be_oblique(&lower) || may_be_oblique(&form_lower) {
        return None;
    }
    let word_stem = stem(&lower);
    let ending = &lower[word_stem.len()..];
    let form_stem = stem(&form_lower);
    let form_ending = &form_lower[form_stem.len()..];
    if ending == form_ending && (word_stem == form_stem || same_word(&word_stem, &form_stem)) {
        return Some(with_case(form, word));
    }
    None
}

/// A form of `from` (`word`, as written in a line) put as `to` in the same
/// case: the cases `word` may stand in by `from`'s rules, each given `to`'s
/// usual ending; None when they disagree (`Амии` is the genitive, the dative
/// or the prepositional: `Маргариты` or `Маргарите`), when `word` is no form
/// of `from`, or when `to` has no case rules (unless it keeps one form, or
/// `word` is the nominative).
pub fn carry_case(word: &str, from: &str, to: &str) -> Option<String> {
    let w = fold(word);
    let f = fold(from);
    let target = fold(to);
    let cases: Vec<usize> = if w == f {
        vec![0]
    } else {
        paradigm(&f)?.cases_of(&w)
    };
    let to_p = paradigm(&target);
    let mut out: Option<String> = None;
    for case in cases {
        let form = match &to_p {
            Some(p) => {
                let kept: String = to.chars().take(p.stem.chars().count()).collect();
                format!("{kept}{}", p.cases[case][0])
            }
            // A word without case rules goes in as it is, unless it reads as another word's oblique case.
            None if may_be_oblique(&target) => return None,
            None if case == 0 || super::matcher::indeclinable(&target) => to.to_owned(),
            None => return None,
        };
        match &out {
            Some(o) if *o != form => return None,
            _ => out = Some(form),
        }
    }
    out.map(|o| with_case(&o, word))
}

/// The nominatives `word` (lower case) may be a case form of by the case
/// rules, the word itself first, then a man's name, then `-а`, then `-я`.
fn nominatives(word: &str) -> Vec<(String, Vec<usize>)> {
    const CUT: &[&str] = &[
        "", "ою", "ой", "ею", "ей", "ом", "ем", "а", "я", "ы", "и", "е", "у", "ю",
    ];
    let mut out: Vec<(String, Vec<usize>)> = Vec::new();
    let mut add = |n: String| {
        if out.iter().any(|(o, _)| *o == n) {
            return;
        }
        if let Some(p) = paradigm(&n) {
            let cases = p.cases_of(word);
            if !cases.is_empty() {
                out.push((n, cases));
            }
        }
    };
    add(word.to_owned());
    for nominative in ["", "а", "я"] {
        for e in CUT {
            if let Some(base) = word.strip_suffix(e)
                && base.chars().count() >= 2
            {
                add(format!("{base}{nominative}"));
            }
        }
    }
    out
}

thread_local! {
    /// The glossary's Russian words (lower case, `ё` = `е`) while a plan is
    /// worked out (`with_known`): the words another word may be a case of.
    static KNOWN: std::cell::RefCell<std::collections::HashSet<String>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

/// Run `f` (a plan, a replacement) knowing the glossary's Russian words, so
/// that a word that may be a known word's oblique case is left to a person.
pub fn with_known<R>(known: std::collections::HashSet<String>, f: impl FnOnce() -> R) -> R {
    KNOWN.with(|k| *k.borrow_mut() = known);
    let out = f();
    KNOWN.with(|k| k.borrow_mut().clear());
    out
}

/// The glossary's Russian words, for `with_known`.
pub fn known_words(terms: &[super::parse::Term]) -> std::collections::HashSet<String> {
    terms
        .iter()
        .flat_map(|t| super::term_forms(t))
        .flat_map(|f| form_folded(&f))
        .collect()
}

/// Whether a word (lower case, `ё` = `е`) may also be an oblique case of
/// another word the glossary has, by the case rules: `зои` (Зоя's
/// genitive, dative, prepositional), `зое` (Зоя's dative) while «Зоя» is a
/// term. Its case in a line cannot be told then, so nothing is put in for it.
pub fn may_be_oblique(word: &str) -> bool {
    KNOWN.with(|k| {
        let known = k.borrow();
        nominatives(word)
            .iter()
            .any(|(n, cases)| n != word && known.contains(n) && cases.iter().any(|&c| c != 0))
    })
}

/// The old and the new word of a proposal (`Бибеку` → `Байбику`, the same
/// case) as nominatives (`Бибек`, `Байбик`), written as they were: the pair
/// whose case rules let both stand in a common case, in the order of
/// `nominatives`. Words without case rules are taken as they are.
pub fn nominative_pair(from: &str, to: &str) -> (String, String) {
    let (f, t) = (fold(from), fold(to));
    for (nf, cf) in nominatives(&f) {
        for (nt, ct) in nominatives(&t) {
            if cf.iter().any(|c| ct.contains(c)) {
                return (with_case(&nf, from), with_case(&nt, to));
            }
        }
    }
    (from.to_owned(), to.to_owned())
}

/// Whether a line's word (lower case, `ё` = `е`) is a form of `from`: the
/// word itself or one of its singular case forms (`paradigm`).
pub fn is_word_of(word: &str, from: &str) -> bool {
    word == from || inflects(word, from)
}

/// `text` with every form of the word `from` replaced by `to` in the same
/// case (only those words; everything else as it is). None when no word of
/// `text` is a form of `from`; Some(None) when one of them cannot be carried
/// over for sure ("check by hand").
pub fn replace_word(text: &str, from: &str, to: &str) -> Option<Option<String>> {
    let from_folded = fold(from);
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut found = false;
    for (s, e) in word_spans(text) {
        let word = &text[s..e];
        if !is_word_of(&fold(word), &from_folded) {
            continue;
        }
        found = true;
        let Some(put) = carry_case(word, from, to) else {
            return Some(None);
        };
        out.push_str(&text[last..s]);
        out.push_str(&put);
        last = e;
    }
    if !found {
        return None;
    }
    out.push_str(&text[last..]);
    Some(Some(out))
}

/// `text` with the words of `hit` that are not the glossary's replaced by
/// the `form` words in the same case; None when one of them cannot be fixed
/// for sure.
pub fn fix_hit(text: &str, hit: &Hit, form_words: &[&str]) -> Option<String> {
    if hit.words.len() != form_words.len() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (w, form) in hit.words.iter().zip(form_words) {
        out.push_str(&text[last..w.start]);
        let word = &text[w.start..w.end];
        if w.ok {
            out.push_str(word);
        } else {
            out.push_str(&fix_word(word, form)?);
        }
        last = w.end;
    }
    out.push_str(&text[last..]);
    Some(out)
}

/// The words of a glossary form as written (what `form_stems` stems).
pub fn form_words(form: &str) -> Vec<&str> {
    form.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !w.eq_ignore_ascii_case("ооо") && w.to_lowercase() != "ооо")
        .collect()
}

/// For each line's `(text, verified)`: None when the line does not write the
/// term as the variant `stem` (or has no text, or is verified by an approval);
/// Some(None) when it does but the case is not sure; Some(Some(text)) with
/// the glossary's form put in.
pub fn plan(
    term: &super::parse::Term,
    stem_key: &str,
    lines: &[(Option<&str>, bool)],
) -> Vec<Option<Option<String>>> {
    let forms = super::term_forms(term);
    let phrase = forms.first().is_some_and(|f| form_stems(f).len() >= 2);
    lines
        .iter()
        .map(|&(text, verified)| {
            let text = text.filter(|_| !verified)?;
            let result = if phrase {
                let (hit, form) = super::phrase_hit(text, &forms)?;
                if hit.key(text) != stem_key || hit.all_ok() {
                    return None;
                }
                fix_hit(text, &hit, &form_words(form))
            } else {
                let (start, end) = root_word(text, stem_key)?;
                let form = forms.first().and_then(|f| form_words(f).first().copied())?;
                // The word is the glossary's already (another ending): nothing to do.
                if is_form_word(&fold(&text[start..end]), &fold(form)) {
                    return None;
                }
                fix_word(&text[start..end], form)
                    .map(|w| format!("{}{}{}", &text[..start], w, &text[end..]))
            };
            Some(result.filter(|r| r != text))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_case_is_carried_over_when_the_words_decline_alike() {
        assert_eq!(fix_word("Рэндаллу", "Рэндолл").as_deref(), Some("Рэндоллу"));
        assert_eq!(fix_word("Рэндалл", "Рэндолл").as_deref(), Some("Рэндолл"));
        assert_eq!(fix_word("РЭНДАЛЛА", "Рэндолл").as_deref(), Some("РЭНДОЛЛА"));
        assert_eq!(
            fix_word("Хофстадтеру", "Хофштадтер").as_deref(),
            Some("Хофштадтеру")
        );
        // The same ending as the form's: the form itself.
        assert_eq!(fix_word("Амийя", "Амия").as_deref(), Some("Амия"));
        // The stems end unlike (й / и): left to a person.
        assert_eq!(fix_word("Амийи", "Амия"), None);
        // Another word (the month for the name): the name may not decline at all.
        assert_eq!(fix_word("апреля", "Эйприл"), None);
        // A vowel stem against a consonant one: not sure.
        assert_eq!(fix_word("Чени", "Чэнь"), None);
    }

    #[test]
    fn a_word_is_replaced_in_its_case() {
        assert_eq!(carry_case("Бибеку", "Бибек", "Байбик").as_deref(), Some("Байбику"));
        assert_eq!(carry_case("Бибеком", "Бибек", "Байбик").as_deref(), Some("Байбиком"));
        assert_eq!(carry_case("БИБЕК", "Бибек", "Байбик").as_deref(), Some("БАЙБИК"));
        // Genitive, dative or prepositional: the target tells them apart, a person decides.
        assert_eq!(carry_case("Амии", "Амия", "Маргарита"), None);
        assert_eq!(carry_case("Амию", "Амия", "Маргарита").as_deref(), Some("Маргариту"));
        assert_eq!(carry_case("Зое", "Зоя", "Майя").as_deref(), Some("Майе"));
        // A target without case rules: only the nominative, or one that never declines.
        assert_eq!(carry_case("Бибека", "Бибек", "Чэнь"), None);
        assert_eq!(carry_case("Бибек", "Бибек", "Чэнь").as_deref(), Some("Чэнь"));
        assert_eq!(carry_case("Бибека", "Бибек", "Сато").as_deref(), Some("Сато"));
        assert_eq!(
            replace_word("Скажи Бибеку, что Бибек прав.", "Бибек", "Байбик"),
            Some(Some("Скажи Байбику, что Байбик прав.".into()))
        );
        assert_eq!(replace_word("Никого нет.", "Бибек", "Байбик"), None);
        assert_eq!(
            nominative_pair("Бибеку", "Байбику"),
            ("Бибек".to_owned(), "Байбик".to_owned())
        );
        assert_eq!(
            nominative_pair("Бибек", "Байбик"),
            ("Бибек".to_owned(), "Байбик".to_owned())
        );
        assert_eq!(
            nominative_pair("Амии", "Маргариты"),
            ("Амия".to_owned(), "Маргарита".to_owned())
        );
        assert_eq!(
            nominative_pair("Чэнь", "Чен"),
            ("Чэнь".to_owned(), "Чен".to_owned())
        );
        assert_eq!(replace_word("У Амии всё.", "Амия", "Маргарита"), Some(None));
        // Only the word: quotes, tags and the rest stay as written.
        assert_eq!(
            replace_word("<i>„Бибек“</i>, — сказал он.", "Бибек", "Байбик"),
            Some(Some("<i>„Байбик“</i>, — сказал он.".into()))
        );
    }

    #[test]
    fn a_word_that_may_be_another_words_case_is_left_to_a_person() {
        // Zoey Perris: «Зое Перрис» in the glossary, «Зои Перрис» in the game;
        // both read as Зоя's oblique cases too — never «Я Зое Перрис».
        let zoya = super::super::parse::Term {
            key: "Zoya".into(),
            ru: "Зоя".into(),
            ..Default::default()
        };
        let zoey = super::super::parse::Term {
            key: "Zoey Perris".into(),
            ru: "Зое Перрис".into(),
            ..Default::default()
        };
        let known = known_words(&[zoya, zoey.clone()]);
        with_known(known, || {
            assert!(may_be_oblique("зои"));
            assert!(may_be_oblique("зое"));
            assert!(!may_be_oblique("зоуи"));
            assert!(!may_be_oblique("рэндолл"));
            assert_eq!(fix_word("Зои", "Зое"), None);
            assert_eq!(
                plan(&zoey, "зои перрис", &[(Some("Я Зои Перрис."), false)]),
                vec![Some(None)]
            );
            assert_eq!(carry_case("Зоуи", "Зоуи", "Зое"), None);
        });
        // Without «Зоя» in the glossary «Зои» is just a name.
        assert!(!may_be_oblique("зои"));
    }

    #[test]
    fn the_nominative_never_takes_another_case() {
        // "Зои" for "Зоуи": the case of "Зои" cannot be told by the form's
        // rules (no ending known for -и), a person decides.
        assert_eq!(fix_word("Зои", "Зоуи"), None);
        // A form with case rules: the ending carried over, never the nominative.
        assert_eq!(fix_word("Зоэи", "Зоя"), None);
        assert_eq!(fix_word("Маргорите", "Маргарита").as_deref(), Some("Маргарите"));
        assert_eq!(fix_word("Маргоритой", "Маргарита").as_deref(), Some("Маргаритой"));
        assert_eq!(fix_word("Маргорита", "Маргарита").as_deref(), Some("Маргарита"));
        assert_eq!(fix_word("Амийей", "Амия"), None);
        assert_eq!(fix_word("Бибеку", "Байбик"), None);
        assert_eq!(fix_word("Байбека", "Байбик").as_deref(), Some("Байбика"));
    }

    #[test]
    fn a_line_already_in_a_case_of_the_form_is_left() {
        let zoe = super::super::parse::Term {
            key: "Zoe".into(),
            ru: "Зоя".into(),
            ..Default::default()
        };
        assert_eq!(plan(&zoe, "зои", &[(Some("У Зои царапина."), false)]), vec![None]);
    }

    #[test]
    fn a_phrase_is_found_by_its_matching_words() {
        let stems = form_folded("Рэндолл Хофштадтер");
        let text = "Письмо для вас, Рэндалл Хофштадтер.";
        let hit = best_window(text, &stems).expect("found");
        assert_eq!(hit.spelling(text), "Рэндалл Хофштадтер");
        assert_eq!(hit.wrong(), vec![0]);
        assert_eq!(
            fix_hit(text, &hit, &form_words("Рэндолл Хофштадтер")).as_deref(),
            Some("Письмо для вас, Рэндолл Хофштадтер.")
        );
        assert!(best_window("Никого нет.", &stems).is_none());
        // Only the surname: the run with it, capitalised neighbours preferred.
        let hit = best_window("Вы Хофштадтер?", &stems).expect("found");
        assert_eq!(hit.spelling("Вы Хофштадтер?"), "Вы Хофштадтер");
    }

    #[test]
    fn a_plan_names_the_sure_lines_and_leaves_the_rest() {
        let term = super::super::parse::Term {
            key: "Randall Hofstadter".into(),
            ru: "Рэндолл Хофштадтер".into(),
            ..Default::default()
        };
        let lines = [
            (Some("Вы Рэндалл Хофштадтер?"), false),
            (Some("Письмо Рэндаллу Хофштадтеру."), false),
            (Some("Господин Рэндолл Хофштадтер."), false),
            (Some("Вы Рэндалл Хофштадтер?"), true),
            (None, false),
        ];
        assert_eq!(
            plan(&term, "рэндалл хофштадтер", &lines),
            vec![
                Some(Some("Вы Рэндолл Хофштадтер?".into())),
                Some(Some("Письмо Рэндоллу Хофштадтеру.".into())),
                None,
                None,
                None
            ]
        );
        let april = super::super::parse::Term {
            key: "April".into(),
            ru: "Эйприл".into(),
            ..Default::default()
        };
        assert_eq!(
            plan(
                &april,
                "апрел",
                &[
                    (Some("Спроси апреля."), false),
                    (Some("Это апрель."), false)
                ]
            ),
            // Another word altogether (the month): both left to a person.
            vec![Some(None), Some(None)]
        );
    }

    #[test]
    fn a_word_is_found_by_its_root() {
        assert_eq!(root_word("Из заражённых.", "заражен"), Some((5, 25)));
        assert_eq!(root_word("Это апреля.", "эйприл"), None);
    }
}
