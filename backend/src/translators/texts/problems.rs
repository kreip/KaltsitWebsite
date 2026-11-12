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
static GARBAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[@$/]").expect("the garbage pattern"));

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
        let close = |out: &mut Markup, open: &mut Vec<(Kind, String)>, kind: Kind, name: Option<&str>| {
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
            push(&mut out, &mut open, Kind::ColorSize, g.as_str().to_lowercase());
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

/// `(brokenTag, noHighlight)` of a translation against its original.
pub fn tag_problems(en: &str, ru: &str) -> (bool, bool) {
    let e = markup(en);
    let r = markup(ru);
    let extra = r
        .counts
        .iter()
        .any(|(name, n)| *n > e.counts.get(name).copied().unwrap_or(0));
    let missing = e
        .counts
        .iter()
        .any(|(name, n)| *n > r.counts.get(name).copied().unwrap_or(0));
    let broken = r.unclosed > e.unclosed || r.garbage > e.garbage || r.tokens != e.tokens || extra;
    (broken, !broken && missing)
}

/// Line breaks as the reader sees them (literal `\n` too), trimmed.
fn breaks(s: &str) -> usize {
    s.replace("\\n", "\n").trim().matches('\n').count()
}

/// Every problem of the game's Russian `ru` of the English `en`.
pub fn problems(en: &str, ru: &str, finder: Option<(&Finder, &[glossary::parse::Term])>) -> Vec<&'static str> {
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
    sqlx::query("UPDATE translators.texts SET problems = '{}' WHERE server = $1 AND problems <> '{}'")
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
        assert_eq!(tag_problems(en, "Наносит <@ba.vup>120%</> урона {0} целям"), (false, false));
        // The highlight lost, the text whole.
        assert_eq!(tag_problems(en, "Наносит 120% урона {0} целям"), (false, true));
        // Left open.
        assert_eq!(tag_problems(en, "Наносит <@ba.vup>120% урона {0} целям"), (true, false));
        // Broken: half a tag.
        assert_eq!(tag_problems(en, "Наносит <@ba.vup120%</> урона {0} целям"), (true, false));
        // A substitution lost.
        assert_eq!(tag_problems(en, "Наносит <@ba.vup>120%</> урона целям"), (true, false));
        // A stray closer.
        assert_eq!(tag_problems("Plain", "Просто</>"), (true, false));
        // Names in angle brackets are text.
        assert_eq!(tag_problems("<Roadblock> ahead", "<Заграждение> впереди"), (false, false));
    }

    #[test]
    fn counts_line_breaks_as_the_reader_sees_them() {
        assert_eq!(problems("A\\nB", "А\nБ", None), Vec::<&str>::new());
        assert_eq!(problems("A\nB", "А Б", None), vec!["newlines"]);
    }
}
