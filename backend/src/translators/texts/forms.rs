//! "Check the gender" and "ты / вы" by the Chinese: marks to look at, not
//! errors (they can be wrong). The page has the very same rules
//! (`frontend/src/components/translators/checks.ts`: `genderOf`,
//! `pronounGender`, `cnAddress`), with the same test cases.
//!
//! * The speaker's gender (an operator's, from the game's files): a Russian
//!   past verb or short adjective (participle) right after «я» of the other
//!   gender — «я сделал» from a woman, «я рада» from a man. Nothing else: no
//!   noun («я профессионал»), pronoun («я сама»: skipped to the verb after
//!   it), name («Я Гавиал») or word of the next sentence.
//! * A Chinese 她 / 他 (she / he; 们 plural and words like 其他 "other" left
//!   out) where the Russian has only the other gender's pronouns.
//! * 您 (polite "you") where the Russian says «ты»; a line with 你们 / 您们
//!   (plural) is left alone. 你 asks for nothing: it is the plain "you", and
//!   «вы» for it is the translator's choice.

use std::collections::HashSet;
use std::sync::LazyLock;

/// `f` or `m`.
pub type Gender = char;

/// Short adjectives and participles after «я»: the feminine, then the masculine.
const SHORT: &[(&str, &str)] = &[
    ("рада", "рад"),
    ("готова", "готов"),
    ("уверена", "уверен"),
    ("должна", "должен"),
    ("согласна", "согласен"),
    ("свободна", "свободен"),
    ("виновата", "виноват"),
    ("права", "прав"),
    ("жива", "жив"),
    ("счастлива", "счастлив"),
    ("благодарна", "благодарен"),
    ("способна", "способен"),
    ("обязана", "обязан"),
    ("знакома", "знаком"),
    ("занята", "занят"),
    ("удивлена", "удивлён"),
    ("удивлена", "удивлен"),
    ("тронута", "тронут"),
    ("рождена", "рождён"),
    ("рождена", "рожден"),
    ("польщена", "польщён"),
    ("польщена", "польщен"),
    ("спокойна", "спокоен"),
    ("больна", "болен"),
    ("здорова", "здоров"),
    ("честна", "честен"),
    ("нужна", "нужен"),
    ("влюблена", "влюблён"),
    ("влюблена", "влюблен"),
    ("измотана", "измотан"),
    ("ранена", "ранен"),
    ("заинтересована", "заинтересован"),
    ("вынуждена", "вынужден"),
    ("поражена", "поражён"),
    ("поражена", "поражен"),
    ("разочарована", "разочарован"),
    ("напугана", "напуган"),
    ("потрясена", "потрясён"),
    ("потрясена", "потрясен"),
];

/// Words skipped between «я» and the word that tells the gender.
const FILLERS: &[&str] = &[
    "не", "уже", "же", "ж", "ведь", "тоже", "бы", "так", "очень", "все", "всё", "еще", "ещё", "просто",
    "только", "сейчас", "правда", "и", "ни", "вот", "даже", "лишь", "никогда", "всегда", "давно",
    "наконец", "точно", "действительно", "совсем", "вовсе", "сегодня", "тогда", "почти", "здесь",
    "тут", "там", "ли", "-", "сам", "сама", "один", "одна",
];

/// Words in «-л» / «-ла» that are no past verbs: the nouns and adverbs of the game's Russian («профессионал»,
/// «генерал», «сначала»). The page has the same list (`notVerbs.ts`): change both.
static NOT_VERBS: LazyLock<HashSet<&str>> =
    LazyLock::new(|| include_str!("not_verbs.txt").split_whitespace().collect());

/// The gender a word tells as the speaker's, right after «я».
fn word_gender(w: &str) -> Option<Gender> {
    for (f, m) in SHORT {
        if *f == w {
            return Some('f');
        }
        if *m == w {
            return Some('m');
        }
    }
    if NOT_VERBS.contains(w) || !w.chars().all(|c| ('а'..='я').contains(&c) || c == 'ё') {
        return None;
    }
    let n = w.chars().count();
    if (w.ends_with("лась") && n >= 6) || (w.ends_with("ла") && n >= 4) {
        return Some('f');
    }
    if (w.ends_with("лся") && n >= 5) || (w.ends_with('л') && n >= 4) || ["был", "шел", "шёл"].contains(&w) {
        return Some('m');
    }
    None
}

/// The text without tags and quoted speech (another's words: «…», “…”).
fn own_speech(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let (mut tag, mut quote) = (false, 0usize);
    for c in text.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if tag => {}
            '«' | '“' => quote += 1,
            '»' | '”' => quote = quote.saturating_sub(1),
            _ if quote > 0 => {}
            _ => out.push(c),
        }
    }
    out
}

fn in_word(c: char) -> bool {
    c.is_alphabetic() || c == '-'
}

/// The speaker's own words in lower case, ё kept.
fn words(text: &str) -> Vec<String> {
    own_speech(text)
        .to_lowercase()
        .split(|c: char| !in_word(c))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

enum Token {
    /// A word in lower case; `named`: written as a name («Гавиал», not «ГОТОВ»).
    Word { lower: String, named: bool },
    /// A full stop: what follows is no longer said of «я».
    Stop,
    /// «…», «!», «?»: the speaker goes on («Я... Ответила»), maybe with a capital.
    Pause,
}

fn tokens(text: &str) -> Vec<Token> {
    let text = own_speech(text);
    let mut out = Vec::new();
    let mut rest = text.as_str();
    while let Some(c) = rest.chars().next() {
        if in_word(c) {
            let end = rest.find(|c: char| !in_word(c)).unwrap_or(rest.len());
            let word = &rest[..end];
            let mut after_first = word.chars().skip(1).peekable();
            let named = c.is_uppercase()
                && after_first.peek().is_some()
                && !after_first.any(char::is_uppercase);
            out.push(Token::Word {
                lower: word.to_lowercase(),
                named,
            });
            rest = &rest[end..];
        } else if c == '.' {
            let end = rest.find(|c: char| c != '.').unwrap_or(rest.len());
            out.push(if end == 1 { Token::Stop } else { Token::Pause });
            rest = &rest[end..];
        } else {
            if matches!(c, '…' | '!' | '?') {
                out.push(Token::Pause);
            }
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// The gender the first telling word after a «я» gives: `after` is what follows it.
fn after_ia(after: &[Token]) -> Option<Gender> {
    let mut paused = false;
    let mut seen = 0;
    for token in after {
        match token {
            Token::Stop => return None,
            Token::Pause => paused = true,
            Token::Word { lower, named } => {
                seen += 1;
                if seen > 4 {
                    return None;
                }
                if FILLERS.contains(&lower.as_str()) {
                    paused = false;
                } else if *named && !paused {
                    return None;
                } else {
                    return word_gender(lower);
                }
            }
        }
    }
    None
}

/// The gender the Russian gives its speaker by «я …», when it gives one.
pub fn speaker_gender_in(ru: &str) -> Option<Gender> {
    let all = tokens(ru);
    let (mut f, mut m) = (0, 0);
    for (i, token) in all.iter().enumerate() {
        if !matches!(token, Token::Word { lower, .. } if lower == "я") {
            continue;
        }
        match after_ia(&all[i + 1..]) {
            Some('f') => f += 1,
            Some(_) => m += 1,
            None => {}
        }
    }
    match (f, m) {
        (f, 0) if f > 0 => Some('f'),
        (0, m) if m > 0 => Some('m'),
        _ => None,
    }
}

/// «One line for operators of different genders»: a text both a woman and a man say whose Russian gives
/// its speaker a gender — wrong for one of them whichever it is.
pub fn mixed_mismatch(ru: &str, mixed: bool) -> bool {
    mixed && speaker_gender_in(ru).is_some()
}

/// «Check the gender» by the speaker: the gender the line should have, when the Russian gives the other.
pub fn speaker_mismatch(ru: &str, speaker: Option<Gender>) -> Option<Gender> {
    let want = speaker?;
    speaker_gender_in(ru).filter(|g| *g != want).map(|_| want)
}

const MASC: &[&str] = &["он", "его", "него", "ему", "нему", "нем", "нём"];
const FEM: &[&str] = &["она", "ее", "её", "нее", "неё", "ей", "ней", "ею", "нею"];

/// Chinese words holding 他 / 她 that are no "he" / "she".
const CN_NOT_PERSON: &[&str] = &[
    "他们", "她们", "其他", "他人", "吉他", "他乡", "他日", "他方", "他国", "利他", "他者", "无他", "排他",
];

/// The gender of the person the Chinese calls 她 / 他, when only one of them stands.
fn cn_pronoun(cn: &str) -> Option<Gender> {
    let mut rest = cn.to_owned();
    for w in CN_NOT_PERSON {
        rest = rest.replace(w, "");
    }
    match (rest.contains('她'), rest.contains('他')) {
        (true, false) => Some('f'),
        (false, true) => Some('m'),
        _ => None,
    }
}

/// «Check the gender» by the Chinese: 她 with only «он / его…» in the Russian, or 他 with only «она / её…».
pub fn pronoun_mismatch(ru: &str, cn: Option<&str>) -> Option<Gender> {
    let want = cn_pronoun(cn?)?;
    let w = words(ru);
    let masc = w.iter().any(|x| MASC.contains(&x.as_str()));
    let fem = w.iter().any(|x| FEM.contains(&x.as_str()));
    match want {
        'f' if masc && !fem => Some('f'),
        'm' if fem && !masc => Some('m'),
        _ => None,
    }
}

const TY: &[&str] = &[
    "ты", "тебя", "тебе", "тобой", "тобою", "твой", "твоя", "твоё", "твое", "твои", "твоего", "твоей",
    "твоему", "твоим", "твоими", "твоих", "твою",
];
const VY: &[&str] = &[
    "вы", "вас", "вам", "вами", "ваш", "ваша", "ваше", "ваши", "вашего", "вашей", "вашему", "вашим",
    "вашими", "ваших", "вашу",
];

/// How the Russian addresses someone: `т` (ты) or `в` (вы), the form used more.
fn address_of(ru: &str) -> Option<char> {
    let w = words(ru);
    let ty = w.iter().filter(|x| TY.contains(&x.as_str())).count();
    let vy = w.iter().filter(|x| VY.contains(&x.as_str())).count();
    match ty.cmp(&vy) {
        std::cmp::Ordering::Greater => Some('т'),
        std::cmp::Ordering::Less => Some('в'),
        std::cmp::Ordering::Equal => None,
    }
}

/// «ты / вы» by the Chinese: the form expected (`т` / `в`) when the Russian uses the other.
pub fn cn_address(ru: &str, cn: Option<&str>) -> Option<char> {
    let cn = cn?;
    if cn.contains("你们") || cn.contains("您们") {
        return None;
    }
    // Only 您 (polite) asks for a form: 你 is the plain "you" of most Chinese lines, and
    // the Russian «вы» for it is a translator's choice (8,350 such marks, nearly all wrong).
    if !cn.contains('您') || cn.contains('你') {
        return None;
    }
    let want = 'в';
    address_of(ru).filter(|a| *a != want).map(|_| want)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same cases as `checks.test.ts` ("gender and address by the Chinese").
    #[test]
    fn the_speaker_gender_by_ia() {
        assert_eq!(speaker_gender_in("Я сделал всё, что мог."), Some('m'));
        assert_eq!(speaker_gender_in("Я так рада тебя видеть!"), Some('f'));
        assert_eq!(speaker_gender_in("Я уже не уверена…"), Some('f'));
        assert_eq!(speaker_gender_in("Я тебя понял."), None);
        assert_eq!(speaker_gender_in("Он сказал: «Я ушёл»."), None);
        assert_eq!(speaker_gender_in("Я — Амия."), None);
        assert_eq!(speaker_mismatch("Я сделал это.", Some('f')), Some('f'));
        assert_eq!(speaker_mismatch("Я сделала это.", Some('f')), None);
        assert_eq!(speaker_mismatch("Я сделал это.", None), None);
    }

    #[test]
    fn only_verbs_and_short_forms_of_the_first_person() {
        assert_eq!(speaker_gender_in("Я готов."), Some('m'));
        assert_eq!(speaker_gender_in("Я занята."), Some('f'));
        assert_eq!(speaker_gender_in("Я не нужна."), Some('f'));
        assert_eq!(speaker_gender_in("Я вернулся."), Some('m'));
        assert_eq!(speaker_gender_in("Я... понял!"), Some('m'));
        assert_eq!(speaker_gender_in("Я? Боялся?"), Some('m'));
        assert_eq!(speaker_gender_in("Я ОБЯЗАН его увидеть."), Some('m'));
        assert_eq!(speaker_gender_in("Я сам пришёл."), Some('m'));
        // Nouns, adverbs, pronouns, names, the next sentence.
        assert_eq!(speaker_gender_in("Вообще-то я профессионал!"), None);
        assert_eq!(speaker_gender_in("Я тоже генерал."), None);
        assert_eq!(speaker_gender_in("Я сначала схожу в доки."), None);
        assert_eq!(speaker_gender_in("Здесь я выступаю как я сама."), None);
        assert_eq!(speaker_gender_in("Я сама разберусь."), None);
        assert_eq!(speaker_gender_in("Я Грейвл, рыцарь."), None);
        assert_eq!(speaker_gender_in("Ты видел больше, чем я. Слышал о городе?"), None);
    }

    #[test]
    fn one_line_for_operators_of_both_genders() {
        assert!(mixed_mismatch("Я готова.", true));
        assert!(mixed_mismatch("Как я и ожидал.", true));
        assert!(!mixed_mismatch("Всё готово.", true));
        assert!(!mixed_mismatch("Я готова.", false));
    }

    #[test]
    fn pronouns_by_the_chinese() {
        assert_eq!(pronoun_mismatch("Он ушёл.", Some("她走了。")), Some('f'));
        assert_eq!(pronoun_mismatch("Она ушла.", Some("她走了。")), None);
        assert_eq!(pronoun_mismatch("Она ушла.", Some("其他人走了。")), None);
        assert_eq!(pronoun_mismatch("Её нет.", Some("他不在。")), Some('m'));
        assert_eq!(pronoun_mismatch("Он и она.", Some("他和她。")), None);
    }

    #[test]
    fn address_by_the_chinese() {
        assert_eq!(cn_address("Ты устал?", Some("您累了吗？")), Some('в'));
        assert_eq!(cn_address("Вы устали?", Some("您累了吗？")), None);
        assert_eq!(cn_address("Вы устали?", Some("你累了吗？")), None);
        assert_eq!(cn_address("Вы устали?", Some("你们累了吗？")), None);
        assert_eq!(cn_address("Устали?", Some("你累了吗？")), None);
    }
}
