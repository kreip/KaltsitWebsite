//! What is wrong with the game's Russian of each text, kept in
//! `texts.problems` for the search's status filter (the tables work the same
//! out on the page, `checks.ts`): `glossary` (a glossary term's form is not
//! found), `brokenTag` (a tag left open or broken, a stray closer, a lost or
//! added substitution `{…}`, a tag the original lacks), `noHighlight` (the
//! original has a tag the Russian lacks, the text otherwise whole),
//! `newlines` (line breaks not as in the English). Redone after every import
//! and glossary download.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use sqlx::PgPool;

use super::super::glossary;
use super::super::glossary::matcher::{Finder, term_found};
use super::plugin_styles;

/// The problem codes, as the search's status filter names them.
pub const PROBLEMS: &[&str] = &["glossary", "brokenTag", "noHighlight", "newlines"];

/// The markup scanner of `markup.ts`: 1 game opener · 2 universal closer ·
/// 3 paragraph · 4 inline opener · 5 inline closer · 6 colour / size opener ·
/// 7 colour / size closer · 8 substitution.
static SCAN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)<([@$][A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)*)>|(</>)|<(p=\d+)>|<(i|b|u)>|</(i|b|u)>|<(color|size)[ =][^<>]*>|</(color|size)>|(\{[^{}<>\n]{1,60}\})",
    )
    .expect("the markup pattern")
});

/// Something that starts like a tag but is not one.
static GARBAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[@$/]").expect("the garbage pattern"));

#[derive(Debug, Default, PartialEq, Eq)]
struct Markup {
    /// Spans by name (`@ba.vup`, `i`, `color`), each span once.
    counts: HashMap<String, usize>,
    /// Spans never closed.
    unclosed: usize,
    /// Closers with nothing to close and tag-like starts that are not tags.
    garbage: usize,
    /// The substitutions, sorted (`{@nickname}` in lower case).
    tokens: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Closed by `</>`: a game style or term, a story paragraph.
    Universal,
    Inline,
    ColorSize,
}

fn markup(text: &str) -> Markup {
    let mut out = Markup::default();
    let mut open: Vec<(Kind, String)> = Vec::new();
    let mut last = 0;
    let push = |out: &mut Markup, open: &mut Vec<(Kind, String)>, kind: Kind, name: String| {
        *out.counts.entry(name.clone()).or_default() += 1;
        open.push((kind, name));
    };
    for c in SCAN.captures_iter(text) {
        let m = c.get(0).expect("the whole match");
        out.garbage += GARBAGE.find_iter(&text[last..m.start()]).count();
        last = m.end();
        // Close the innermost span that matches; the ones opened inside it end unclosed.
        let close =
            |out: &mut Markup, open: &mut Vec<(Kind, String)>, kind: Kind, name: Option<&str>| {
                match open
                    .iter()
                    .rposition(|(k, n)| *k == kind && name.is_none_or(|x| x == n))
                {
                    Some(i) => {
                        out.unclosed += open.len() - i - 1;
                        open.truncate(i);
                    }
                    None => out.garbage += 1,
                }
            };
        if let Some(g) = c.get(1) {
            push(&mut out, &mut open, Kind::Universal, g.as_str().to_owned());
        } else if c.get(2).is_some() {
            close(&mut out, &mut open, Kind::Universal, None);
        } else if c.get(3).is_some() {
            push(&mut out, &mut open, Kind::Universal, "p".to_owned());
        } else if let Some(g) = c.get(4) {
            push(&mut out, &mut open, Kind::Inline, g.as_str().to_lowercase());
        } else if let Some(g) = c.get(5) {
            let name = g.as_str().to_lowercase();
            close(&mut out, &mut open, Kind::Inline, Some(&name));
        } else if let Some(g) = c.get(6) {
            push(
                &mut out,
                &mut open,
                Kind::ColorSize,
                g.as_str().to_lowercase(),
            );
        } else if let Some(g) = c.get(7) {
            let name = g.as_str().to_lowercase();
            close(&mut out, &mut open, Kind::ColorSize, Some(&name));
        } else if let Some(g) = c.get(8) {
            out.tokens.push(g.as_str().to_lowercase());
        }
    }
    out.garbage += GARBAGE.find_iter(&text[last..]).count();
    out.unclosed += open.len();
    out.tokens.sort();
    out
}

/// A span of the original with its text's place in the plain text (tags
/// removed, substitutions kept): `kind` is `style`, `term`, `p`, `inline`
/// or `color` / `size`; `name` names it as the counts do (`@ba.vup`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlainSpan {
    kind: &'static str,
    name: String,
    start: usize,
    end: usize,
}

/// The plain text of `text` and where each span stands in it (an unclosed
/// span ends at the end; a closer with nothing to close is text).
fn plain_spans(text: &str) -> (String, Vec<PlainSpan>) {
    let mut plain = String::with_capacity(text.len());
    let mut open: Vec<PlainSpan> = Vec::new();
    let mut done: Vec<PlainSpan> = Vec::new();
    let mut last = 0;
    let close = |open: &mut Vec<PlainSpan>,
                 done: &mut Vec<PlainSpan>,
                 at: usize,
                 matches: &dyn Fn(&PlainSpan) -> bool|
     -> bool {
        match open.iter().rposition(|s| matches(s)) {
            Some(i) => {
                for mut s in open.drain(i..) {
                    s.end = at;
                    done.push(s);
                }
                true
            }
            None => false,
        }
    };
    for c in SCAN.captures_iter(text) {
        let m = c.get(0).expect("the whole match");
        plain.push_str(&text[last..m.start()]);
        last = m.end();
        let at = plain.len();
        let start = |open: &mut Vec<PlainSpan>, kind: &'static str, name: String| {
            open.push(PlainSpan {
                kind,
                name,
                start: at,
                end: at,
            });
        };
        if let Some(g) = c.get(1) {
            let name = g.as_str().to_owned();
            start(
                &mut open,
                if name.starts_with('$') {
                    "term"
                } else {
                    "style"
                },
                name,
            );
        } else if c.get(2).is_some() {
            if !close(&mut open, &mut done, at, &|s| {
                matches!(s.kind, "style" | "term" | "p")
            }) {
                plain.push_str(m.as_str());
            }
        } else if c.get(3).is_some() {
            start(&mut open, "p", "p".to_owned());
        } else if let Some(g) = c.get(4) {
            start(&mut open, "inline", g.as_str().to_lowercase());
        } else if let Some(g) = c.get(5) {
            let name = g.as_str().to_lowercase();
            if !close(&mut open, &mut done, at, &|s| {
                s.kind == "inline" && s.name == name
            }) {
                plain.push_str(m.as_str());
            }
        } else if let Some(g) = c.get(6) {
            start(&mut open, "colorsize", g.as_str().to_lowercase());
        } else if let Some(g) = c.get(7) {
            let name = g.as_str().to_lowercase();
            if !close(&mut open, &mut done, at, &|s| {
                s.kind == "colorsize" && s.name == name
            }) {
                plain.push_str(m.as_str());
            }
        } else if let Some(g) = c.get(8) {
            plain.push_str(g.as_str());
        }
    }
    plain.push_str(&text[last..]);
    let end = plain.len();
    for mut s in open {
        s.end = end;
        done.push(s);
    }
    (plain, done)
}

/// A number as the plugin finds one to colour again (`230%`, `1.25`,
/// full-width digits too), or a substitution standing for one.
static NUMBER_OR_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{[^{}<>\n]{1,60}\}|[0-9０-９]+(?:[.．][0-9０-９]+)?[%％]?")
        .expect("the number pattern")
});

fn is_ascii_word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric())
}

/// Where `value` stands in `text`, as the plugin looks: an ASCII word only whole.
fn find_all(text: &str, value: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if value.is_empty() {
        return out;
    }
    let mut from = 0;
    while let Some(i) = text[from..].find(value) {
        let at = from + i;
        let end = at + value.len();
        let before = at > 0
            && is_ascii_word(value.chars().next())
            && is_ascii_word(text[..at].chars().next_back());
        let after = end < text.len()
            && is_ascii_word(value.chars().next_back())
            && is_ascii_word(text[end..].chars().next());
        if !before && !after {
            out.push((at, end));
        }
        from = end;
    }
    out
}

/// Full-width digits, dot and percent as their ASCII (the plugin compares numbers in NFKC).
fn ascii_number(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '０'..='９' => {
                char::from_u32(u32::from(c) - u32::from('０') + u32::from('0')).unwrap_or(c)
            }
            '．' => '.',
            '％' => '%',
            c => c,
        })
        .collect()
}

/// Where a number (or a substitution) is in `text`, by its digits.
fn find_numbers(text: &str, value: &str) -> Vec<(usize, usize)> {
    let key = ascii_number(value);
    NUMBER_OR_TOKEN
        .find_iter(text)
        .filter(|m| ascii_number(m.as_str()) == key)
        .map(|m| (m.start(), m.end()))
        .collect()
}

/// Whether the RichTextFix plugin puts the span's style back into a plain
/// translation (its `RichTextStyleRestorer`): the style covers the whole
/// text; or the span's text stands in the translation as it is (a number,
/// `SP`) as many times as in the original and only inside the span; or a
/// number (a substitution counts as one) of the span stands in both as many
/// times. A game style the plugin does not know is dropped before that.
fn plugin_restores(s: &PlainSpan, plain: &str, translation: &str) -> bool {
    if s.kind == "p" {
        return false;
    }
    // The game shows a term's words plain (no colour, no underline): nothing to lose.
    if s.kind == "term" {
        return true;
    }
    if s.kind == "style" && !plugin_styles::known(&s.name[1..]) {
        return false;
    }
    let text = plain[s.start..s.end].trim();
    if text.is_empty() {
        return false;
    }
    if text == plain.trim() {
        return true;
    }
    let inside = |hits: &[(usize, usize)]| {
        !hits.is_empty() && hits.iter().all(|&(a, b)| a >= s.start && b <= s.end)
    };
    let literal = find_all(plain, text);
    if inside(&literal) && literal.len() == find_all(translation, text).len() {
        return true;
    }
    let mut seen = std::collections::HashSet::new();
    for n in NUMBER_OR_TOKEN.find_iter(text) {
        if !seen.insert(n.as_str()) {
            continue;
        }
        let hits = find_numbers(plain, n.as_str());
        if inside(&hits) && hits.len() == find_numbers(translation, n.as_str()).len() {
            return true;
        }
    }
    false
}

/// The original's spans whose highlight the game would show without, given
/// the translation: the translation lacks the tag and the plugin would not
/// put it back. A translation with tags of its own gets only whole-text
/// styles back from the plugin; a plain one, what `plugin_restores` says. A
/// term `<$…>` is never one: the game shows its words plain.
pub fn lost_highlights(original: &str, translation: &str) -> Vec<String> {
    let (plain, spans) = plain_spans(original);
    if spans.is_empty() {
        return Vec::new();
    }
    let ru = markup(translation);
    let whole =
        |s: &PlainSpan| !plain.trim().is_empty() && plain[s.start..s.end].trim() == plain.trim();
    if !ru.counts.is_empty() {
        let mut out = Vec::new();
        let mut names: Vec<&str> = spans.iter().map(|s| s.name.as_str()).collect();
        names.dedup();
        names.sort_unstable();
        names.dedup();
        for name in names {
            if name.starts_with('$') {
                continue;
            }
            let list: Vec<&PlainSpan> = spans.iter().filter(|s| s.name == name).collect();
            let missing = list
                .len()
                .saturating_sub(ru.counts.get(name).copied().unwrap_or(0));
            let lost = missing.saturating_sub(list.iter().filter(|s| whole(s)).count());
            out.extend(std::iter::repeat_n(name.to_owned(), lost));
        }
        return out;
    }
    spans
        .iter()
        .filter(|s| !plugin_restores(s, &plain, translation))
        .map(|s| s.name.clone())
        .collect()
}

/// `(brokenTag, noHighlight)` of a translation against its original.
pub fn tag_problems(en: &str, ru: &str) -> (bool, bool) {
    let e = markup(en);
    let r = markup(ru);
    let extra = r
        .counts
        .iter()
        .any(|(name, n)| *n > e.counts.get(name).copied().unwrap_or(0));
    let broken = r.unclosed > e.unclosed || r.garbage > e.garbage || r.tokens != e.tokens || extra;
    (broken, !broken && !lost_highlights(en, ru).is_empty())
}

/// Line breaks as the reader sees them (literal `\n` too), trimmed.
fn breaks(s: &str) -> usize {
    s.replace("\\n", "\n").trim().matches('\n').count()
}

/// Every problem of the game's Russian `ru` of the English `en`.
pub fn problems(
    en: &str,
    ru: &str,
    finder: Option<(&Finder, &[glossary::parse::Term])>,
) -> Vec<&'static str> {
    let mut out = Vec::new();
    if let Some((finder, terms)) = finder {
        // Terms by the English they matched: one of them found is enough (as on the page).
        let mut groups: Vec<(&str, Vec<usize>)> = Vec::new();
        for f in finder.find(en) {
            let matched = &en[f.start..f.end];
            for t in f.terms {
                if terms[t].from_sova {
                    continue;
                }
                match groups.iter_mut().find(|g| g.0 == matched) {
                    Some(g) => g.1.push(t),
                    None => groups.push((matched, vec![t])),
                }
            }
        }
        if groups
            .iter()
            .any(|(_, ts)| !ts.iter().any(|&t| term_found(ru, &terms[t])))
        {
            out.push("glossary");
        }
    }
    let (broken, no_highlight) = tag_problems(en, ru);
    if broken {
        out.push("brokenTag");
    }
    if no_highlight {
        out.push("noHighlight");
    }
    if breaks(en) != breaks(ru) {
        out.push("newlines");
    }
    out
}

/// Work every text's problems out again (after an import or a glossary download).
pub async fn refresh(pool: &PgPool, server: &str) -> Result<usize, sqlx::Error> {
    let stored = glossary::load(pool).await?;
    let terms: Vec<_> = stored.iter().map(glossary::StoredTerm::as_term).collect();
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, en, ru FROM translators.texts WHERE server = $1 AND ru IS NOT NULL",
    )
    .bind(server)
    .fetch_all(pool)
    .await?;
    let found = tokio::task::spawn_blocking(move || {
        let finder = (!terms.is_empty()).then(|| Finder::new(&terms));
        rows.into_iter()
            .filter_map(|(id, en, ru)| {
                let p = problems(&en, &ru, finder.as_ref().map(|f| (f, terms.as_slice())));
                (!p.is_empty()).then(|| (id, p.join(",")))
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE translators.texts SET problems = '{}' WHERE server = $1 AND problems <> '{}'",
    )
    .bind(server)
    .execute(&mut *tx)
    .await?;
    for chunk in found.chunks(5_000) {
        let ids: Vec<i64> = chunk.iter().map(|(id, _)| *id).collect();
        let list: Vec<&str> = chunk.iter().map(|(_, p)| p.as_str()).collect();
        sqlx::query(
            "UPDATE translators.texts t SET problems = string_to_array(x.p, ',')
             FROM UNNEST($1::bigint[], $2::text[]) AS x(id, p) WHERE t.id = x.id",
        )
        .bind(&ids)
        .bind(&list)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    tracing::info!(texts = found.len(), "translators problems redone");
    Ok(found.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_a_broken_tag_from_a_missing_one() {
        let en = "Deals <@ba.vup>120%</> damage to {0} targets";
        // As in the original.
        assert_eq!(
            tag_problems(en, "Наносит <@ba.vup>120%</> урона {0} целям"),
            (false, false)
        );
        // The highlight dropped, but the plugin colours the number again.
        assert_eq!(
            tag_problems(en, "Наносит 120% урона {0} целям"),
            (false, false)
        );
        // A word's highlight the plugin cannot put back.
        assert_eq!(
            tag_problems("<@ba.vup>Stealth</> for {0}", "Невидимость на {0}"),
            (false, true)
        );
        // Left open.
        assert_eq!(
            tag_problems(en, "Наносит <@ba.vup>120% урона {0} целям"),
            (true, false)
        );
        // Broken: half a tag.
        assert_eq!(
            tag_problems(en, "Наносит <@ba.vup120%</> урона {0} целям"),
            (true, false)
        );
        // A substitution lost.
        assert_eq!(
            tag_problems(en, "Наносит <@ba.vup>120%</> урона целям"),
            (true, false)
        );
        // A stray closer.
        assert_eq!(tag_problems("Plain", "Просто</>"), (true, false));
        // Names in angle brackets are text.
        assert_eq!(
            tag_problems("<Roadblock> ahead", "<Заграждение> впереди"),
            (false, false)
        );
    }

    // The same examples stand in frontend/src/components/translators/markup.test.ts.
    #[test]
    fn knows_which_highlights_the_plugin_puts_back() {
        let lost = |en: &str, ru: &str| lost_highlights(en, ru);
        // Kolooo's skill: the numbers come back blue; a term's words are plain in the game anyway.
        let skill = "Deals <@ba.vup>230%</> ATK as Physical damage, <$ba.stun>Stuns</> the target for <@ba.vup>1.25</> seconds";
        let ru = "Наносит 230% АТК физического урона, оглушая цель на 1.25 секунд";
        assert!(lost(skill, ru).is_empty());
        assert_eq!(tag_problems(skill, ru), (false, false));
        assert_eq!(
            lost("<$ba.stun>Stuns</> <i>all</i>", "Оглушает всех"),
            vec!["i"]
        );
        // Without the term, nothing is lost; a number written otherwise (1,25) is.
        let plain = "Deals <@ba.vup>230%</> ATK for <@ba.vup>1.25</> seconds";
        assert!(lost(plain, "Наносит 230% АТК на 1.25 секунд").is_empty());
        assert_eq!(
            lost(plain, "Наносит 230% АТК на 1,25 секунд"),
            vec!["@ba.vup"]
        );
        // A substitution stands for a number; a number of the span that is also outside it cannot be placed.
        assert!(lost("ATK <@ba.vup>+{atk:0%}</>", "Атака +{atk:0%}").is_empty());
        assert_eq!(lost("<@ba.vup>5</> of 5", "5 из 5"), vec!["@ba.vup"]);
        // A style over the whole text comes back; a word does not; a style the plugin does not know is dropped.
        assert!(
            lost(
                "<@ba.rem>Can store 2 charges</>",
                "Может накапливать 2 заряда"
            )
            .is_empty()
        );
        assert_eq!(
            lost("Gains <@ba.vup>Stealth</> now", "Получает невидимость"),
            vec!["@ba.vup"]
        );
        assert_eq!(
            lost("<@act.timeLimit>3 days</> left", "Осталось 3 дня"),
            vec!["@act.timeLimit"]
        );
        // An ASCII word is found whole only.
        assert!(lost("Gains <@ba.vup>SP</> faster", "Быстрее получает SP").is_empty());
        assert_eq!(
            lost("Gains <@ba.vup>SP</> faster", "Быстрее получает SPD"),
            vec!["@ba.vup"]
        );
        // A translation with tags of its own gets only whole-text styles back.
        assert_eq!(
            lost(
                "<i>All</i> of <@ba.vup>10</> and <@ba.vup>20</>",
                "<i>Всё</i> из 10 и <@ba.vup>20</>"
            ),
            vec!["@ba.vup"]
        );
        assert!(lost("<@ba.rem>All <@ba.vup>10</></>", "Всё <@ba.vup>10</>").is_empty());
    }

    #[test]
    fn counts_line_breaks_as_the_reader_sees_them() {
        assert_eq!(problems("A\\nB", "А\nБ", None), Vec::<&str>::new());
        assert_eq!(problems("A\nB", "А Б", None), vec!["newlines"]);
    }
}
