//! A term's Russian in a line against the glossary's form, word by word, and
//! the glossary's form put into the line in the same case ("Рэндаллу" →
//! "Рэндоллу"): the "Replace with the glossary's form" button of the Glossary
//! tab and the "As in the game" column for a term of several words.
//!
//! The case is carried over by the ending the stem rule cuts (`matcher::stem`):
//! the glossary's stem plus the game word's ending. That is sure only when the
//! two words are the same word spelled differently — the game word is in the
//! nominative (no ending cut), or its ending is the glossary form's own, or
//! the stems differ by a letter or two and end alike (both in a consonant, or
//! in the same letter). A different word ("апреля" for "Эйприл": the name
//! does not decline, the month does) is left to a person.

use super::matcher::{stem, word_spans};

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

/// Where a form of `stems.len()` words stands in `text`: the run of as many
/// words with the most words matching the form by stem, position by position
/// (at least one); ties go to the run with more capitalised words, then the
/// earliest. None when no word of the form is in the text.
pub fn best_window(text: &str, stems: &[String]) -> Option<Hit> {
    if stems.is_empty() {
        return None;
    }
    let spans = word_spans(text);
    let mut best: Option<(usize, usize, Hit)> = None;
    for win in spans.windows(stems.len()) {
        let words: Vec<WordHit> = win
            .iter()
            .zip(stems)
            .map(|(&(start, end), s)| WordHit {
                start,
                end,
                ok: fold(&text[start..end]).starts_with(s.as_str()),
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

fn is_consonant(c: char) -> bool {
    matches!(
        c,
        'б' | 'в'
            | 'г'
            | 'д'
            | 'ж'
            | 'з'
            | 'к'
            | 'л'
            | 'м'
            | 'н'
            | 'п'
            | 'р'
            | 'с'
            | 'т'
            | 'ф'
            | 'х'
            | 'ц'
            | 'ч'
            | 'ш'
            | 'щ'
    )
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

/// The glossary's `form` word in the case of the game's `word`, or None
/// when the case cannot be carried over for sure.
pub fn fix_word(word: &str, form: &str) -> Option<String> {
    let lower = fold(word);
    let word_stem = stem(&lower);
    let ending = &lower[word_stem.len()..];
    let form_lower = fold(form);
    let form_stem = stem(&form_lower);
    let form_ending = &form_lower[form_stem.len()..];
    if ending.is_empty() || ending == form_ending {
        return Some(with_case(form, word));
    }
    // A form in a soft sign may not decline at all (Чэнь): a person decides.
    if form_ending == "ь" {
        return None;
    }
    let last = |s: &str| s.chars().next_back();
    match (last(&word_stem), last(&form_stem)) {
        (Some(a), Some(b))
            if (a == b || (is_consonant(a) && is_consonant(b)))
                && same_word(&word_stem, &form_stem) =>
        {
            // The form as written up to its stem, then the game word's ending.
            let kept: String = form.chars().take(form_stem.chars().count()).collect();
            Some(with_case(&format!("{kept}{ending}"), word))
        }
        _ => None,
    }
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
                if fold(&text[start..end]).starts_with(stem(&fold(form)).as_str()) {
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
    fn a_phrase_is_found_by_its_matching_words() {
        let stems = form_stems("Рэндолл Хофштадтер");
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
