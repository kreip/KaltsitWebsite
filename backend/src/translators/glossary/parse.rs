//! The team glossary (a Google Sheet, read as its xlsx export) into terms.
//!
//! Sheet and cell rules (translators/README.md, "XUnity dictionary and plugin
//! format"): sheets found by name, case-insensitive (Sova's by its prefix, Excel
//! cuts names at 31 characters); the first row is a header; rows without
//! Russian are skipped; every cell is trimmed; empty sheets are not an error.
//!
//! * `A / B` in a cell is several English keys for one Russian;
//! * on Sova's sheet only, `A, B` → `X, Y` pairs up when the counts match,
//!   else every English key gets the whole Russian;
//! * `Amiya (Guard)`: the key is `Amiya`, the bracket is shown beside it.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};

/// How a sheet is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SheetRule {
    /// Lower-case name, or prefix when `prefix`.
    name: &'static str,
    prefix: bool,
    /// Terms match only with the case as written (names, classes, races).
    case_sensitive: bool,
    from_sova: bool,
    /// Column of the Russian plural, if the sheet has one.
    plural: Option<usize>,
    /// Columns of the note (shown beside the term) and the comment.
    note: Option<usize>,
    comment: Option<usize>,
    cn: Option<usize>,
}

const SHEETS: &[SheetRule] = &[
    SheetRule {
        name: "игровые персонажи",
        prefix: false,
        case_sensitive: true,
        from_sova: false,
        plural: None,
        note: None,
        comment: Some(4),
        cn: None,
    },
    SheetRule {
        name: "классы и подклассы",
        prefix: false,
        case_sensitive: true,
        from_sova: false,
        plural: None,
        note: Some(2),
        comment: Some(4),
        cn: Some(3),
    },
    SheetRule {
        name: "расы и не только",
        prefix: false,
        case_sensitive: true,
        from_sova: false,
        plural: Some(2),
        note: None,
        comment: Some(3),
        cn: None,
    },
    SheetRule {
        name: "различные термины",
        prefix: false,
        case_sensitive: false,
        from_sova: false,
        plural: None,
        note: None,
        comment: Some(2),
        cn: None,
    },
    SheetRule {
        name: "организации",
        prefix: false,
        case_sensitive: true,
        from_sova: false,
        plural: None,
        note: None,
        comment: Some(2),
        cn: None,
    },
    SheetRule {
        name: "страны, города и локации",
        prefix: false,
        case_sensitive: true,
        from_sova: false,
        plural: None,
        note: None,
        comment: Some(2),
        cn: None,
    },
    SheetRule {
        name: "украдено у совы",
        prefix: true,
        case_sensitive: true,
        from_sova: true,
        plural: None,
        note: Some(2),
        comment: Some(4),
        cn: None,
    },
];

/// One glossary entry: one row's English (one of its variants) and its Russian.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Term {
    pub sheet: String,
    /// 1-based row in the sheet, for "open in the sheet".
    pub row: u32,
    /// The English as written in the cell (all variants).
    pub en: String,
    /// The search key: one English variant without its bracket.
    pub key: String,
    /// The bracket after the key (`Guard` in `Amiya (Guard)`), if any.
    pub qualifier: Option<String>,
    pub ru: String,
    pub ru_plural: Option<String>,
    pub note: Option<String>,
    pub synonyms: Option<String>,
    pub comment: Option<String>,
    pub cn: Option<String>,
    pub case_sensitive: bool,
    pub from_sova: bool,
}

fn cell(row: &[Data], i: usize) -> String {
    row.get(i)
        .map(|d| d.to_string().trim().to_owned())
        .unwrap_or_default()
}

fn opt(row: &[Data], i: Option<usize>) -> Option<String> {
    i.map(|i| cell(row, i)).filter(|s| !s.is_empty())
}

/// `Amiya (Guard)` → (`Amiya`, `Guard`).
pub fn split_bracket(s: &str) -> (String, Option<String>) {
    match (s.find('('), s.rfind(')')) {
        (Some(open), Some(close)) if close > open => {
            let key = format!("{} {}", s[..open].trim(), s[close + 1..].trim())
                .trim()
                .to_owned();
            let q = s[open + 1..close].trim().to_owned();
            (key, (!q.is_empty()).then_some(q))
        }
        _ => (s.trim().to_owned(), None),
    }
}

fn rule_for(sheet: &str) -> Option<SheetRule> {
    let lower = sheet.trim().to_lowercase();
    SHEETS.iter().copied().find(|r| {
        if r.prefix {
            lower.starts_with(r.name)
        } else {
            lower == r.name
        }
    })
}

/// Every English key of a cell with the Russian each one gets.
fn pairs(en: &str, ru: &str, split_commas: bool) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for part in en.split(" / ") {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if split_commas && part.contains(", ") {
            let ens: Vec<&str> = part.split(", ").map(str::trim).collect();
            let rus: Vec<&str> = ru.split(", ").map(str::trim).collect();
            if ens.len() == rus.len() {
                out.extend(
                    ens.iter()
                        .zip(&rus)
                        .map(|(e, r)| ((*e).to_owned(), (*r).to_owned())),
                );
            } else {
                out.extend(ens.iter().map(|e| ((*e).to_owned(), ru.to_owned())));
            }
        } else {
            out.push((part.to_owned(), ru.to_owned()));
        }
    }
    out
}

/// Read every term from the xlsx bytes.
pub fn parse(xlsx: &[u8]) -> anyhow::Result<Vec<Term>> {
    let mut book: Xlsx<_> = Xlsx::new(Cursor::new(xlsx))?;
    let mut out = Vec::new();
    for sheet in book.sheet_names() {
        let Some(rule) = rule_for(&sheet) else {
            continue;
        };
        let range = book.worksheet_range(&sheet)?;
        let first_row = range.start().map_or(0, |(r, _)| r);
        for (i, row) in range.rows().enumerate().skip(1) {
            let en = cell(row, 0);
            let ru = cell(row, 1);
            if en.is_empty() || ru.is_empty() {
                continue;
            }
            let row_no = first_row + u32::try_from(i).unwrap_or(0) + 1;
            let plural = opt(row, rule.plural);
            let base = Term {
                sheet: sheet.trim().to_owned(),
                row: row_no,
                en: en.clone(),
                ru_plural: plural,
                note: opt(row, rule.note),
                synonyms: if rule.from_sova {
                    opt(row, Some(3))
                } else {
                    None
                },
                comment: opt(row, rule.comment),
                cn: opt(row, rule.cn),
                case_sensitive: rule.case_sensitive,
                from_sova: rule.from_sova,
                ..Term::default()
            };
            for (key, ru) in pairs(&en, &ru, rule.from_sova) {
                let (key, qualifier) = split_bracket(&key);
                if key.chars().count() < 2 {
                    continue;
                }
                out.push(Term {
                    key,
                    qualifier,
                    ru,
                    ..base.clone()
                });
            }
            // The characters sheet carries a real name beside some call signs.
            if rule.name == "игровые персонажи" {
                let (real_en, real_ru) = (cell(row, 2), cell(row, 3));
                if !real_en.is_empty() && !real_ru.is_empty() {
                    let (key, qualifier) = split_bracket(&real_en);
                    if key.chars().count() >= 2 {
                        out.push(Term {
                            en: real_en.clone(),
                            key,
                            qualifier,
                            ru: real_ru,
                            ..base.clone()
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brackets_become_qualifiers() {
        assert_eq!(
            split_bracket("Amiya (Guard)"),
            ("Amiya".to_owned(), Some("Guard".to_owned()))
        );
        assert_eq!(split_bracket("Arts "), ("Arts".to_owned(), None));
    }

    #[test]
    fn slash_variants_and_sova_commas() {
        assert_eq!(
            pairs("Elfins / Spirits", "Эльфины", false),
            vec![
                ("Elfins".into(), "Эльфины".into()),
                ("Spirits".into(), "Эльфины".into())
            ]
        );
        assert_eq!(
            pairs(
                "Lungmen Guard Department, LGD",
                "Департамент Охраны Лунмэня, Д. О. Л.",
                true
            ),
            vec![
                (
                    "Lungmen Guard Department".into(),
                    "Департамент Охраны Лунмэня".into()
                ),
                ("LGD".into(), "Д. О. Л.".into())
            ]
        );
        assert_eq!(
            pairs("A, B", "Один", true),
            vec![("A".into(), "Один".into()), ("B".into(), "Один".into())]
        );
        // Commas split only on Sova's sheet.
        assert_eq!(pairs("Rhodes Island, Inc.", "Родос", false).len(), 1);
    }

    #[test]
    fn sheets_are_found_by_name_and_prefix() {
        assert!(rule_for("Игровые персонажи").is_some());
        assert!(rule_for("Украдено у Совы, посмотреть, чт").is_some_and(|r| r.from_sova));
        assert!(rule_for("Трекер сюжетов").is_none());
        assert!(rule_for("ТыВы").is_none());
        assert!(rule_for("Различные термины").is_some_and(|r| !r.case_sensitive));
    }
}

#[cfg(test)]
mod real_file {
    #[test]
    #[ignore = "needs TRANSLATORS_GLOSSARY_XLSX"]
    fn parses_the_real_export() {
        let path = std::env::var("TRANSLATORS_GLOSSARY_XLSX").unwrap();
        let terms = super::parse(&std::fs::read(path).unwrap()).unwrap();
        let mut by_sheet = std::collections::BTreeMap::<&str, usize>::new();
        for t in &terms {
            *by_sheet.entry(t.sheet.as_str()).or_default() += 1;
        }
        eprintln!("{by_sheet:#?}");
        for t in terms
            .iter()
            .filter(|t| t.qualifier.is_some() || t.ru_plural.is_some())
            .take(8)
        {
            eprintln!("{t:?}");
        }
    }
}
