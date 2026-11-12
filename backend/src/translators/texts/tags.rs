//! The markup tags of the game's texts: the style spans (`<@ba.vup>…</>`,
//! coloured from the game's `RichTextStyles`) and the terms (`<$ba.stun>…</>`,
//! which the game explains in a popup: `TermDescriptionDict`). The import
//! lists them with the pack's Russian for a term's name and description and
//! with how many texts use each; the section names them in the editor's
//! palette and shows a term's explanation over its span.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;
use utoipa::ToSchema;

use super::TextRow;
use crate::translators::import::lines::Matcher;

/// One tag of the game's markup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameTag {
    /// With its sigil: `@ba.vup` (a style span), `$ba.stun` (a term).
    pub tag: String,
    /// `style` or `term`.
    pub kind: String,
    /// A style's colour in the game (`#0098DC`), when the game's table has it.
    pub color: Option<String>,
    /// A term's name as the game shows it, and in the pack's Russian.
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    /// A term's explanation, and in the pack's Russian.
    pub description_en: Option<String>,
    pub description_ru: Option<String>,
    /// Texts whose English holds it.
    pub uses: i32,
}

static TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<([@$][A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)*)>").expect("the tag pattern")
});
static COLOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<color=(#[0-9A-Fa-f]{3,8})>").expect("the colour pattern"));

/// The tags a text holds, each once.
pub fn tags_of(text: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for m in TAG.captures_iter(text) {
        if let Some(tag) = m.get(1).map(|g| g.as_str())
            && !out.contains(&tag)
        {
            out.push(tag);
        }
    }
    out
}

fn entries<'a>(table: &'a Value, name: &str) -> impl Iterator<Item = (&'a str, &'a Value)> {
    table
        .get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| Some((e.get("key")?.as_str()?, e.get("value")?)))
}

/// Every tag the texts use or the game's tables define, the most used first.
pub fn collect(excel_dir: &Path, matcher: &Matcher<'_>, rows: &[TextRow]) -> Vec<GameTag> {
    let consts: Value = std::fs::read(excel_dir.join("gamedata_const.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null);
    let mut uses: HashMap<&str, i32> = HashMap::new();
    for r in rows {
        for tag in tags_of(&r.en) {
            *uses.entry(tag).or_default() += 1;
        }
    }
    let ru = |en: &str| matcher.entry(en).map(|(_, ru, _)| ru.to_owned());
    let mut out: HashMap<String, GameTag> = HashMap::new();
    for (key, style) in entries(&consts, "RichTextStyles") {
        let tag = format!("@{key}");
        let color = style
            .as_str()
            .and_then(|s| COLOR.captures(s))
            .and_then(|c| c.get(1))
            .map(|c| c.as_str().to_owned());
        out.insert(
            tag.clone(),
            GameTag {
                uses: uses.get(tag.as_str()).copied().unwrap_or(0),
                tag,
                kind: "style".to_owned(),
                color,
                name_en: None,
                name_ru: None,
                description_en: None,
                description_ru: None,
            },
        );
    }
    for (key, term) in entries(&consts, "TermDescriptionDict") {
        let tag = format!("${key}");
        let text = |name: &str| {
            term.get(name)
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
        };
        let name_en = text("TermName");
        let description_en = text("Description");
        out.insert(
            tag.clone(),
            GameTag {
                uses: uses.get(tag.as_str()).copied().unwrap_or(0),
                tag,
                kind: "term".to_owned(),
                color: None,
                name_ru: name_en.as_deref().and_then(ru),
                description_ru: description_en.as_deref().and_then(ru),
                name_en,
                description_en,
            },
        );
    }
    // A tag the texts use and no table defines.
    for (tag, n) in &uses {
        out.entry((*tag).to_owned()).or_insert_with(|| GameTag {
            tag: (*tag).to_owned(),
            kind: if tag.starts_with('$') {
                "term"
            } else {
                "style"
            }
            .to_owned(),
            color: None,
            name_en: None,
            name_ru: None,
            description_en: None,
            description_ru: None,
            uses: *n,
        });
    }
    let mut out: Vec<GameTag> = out.into_values().collect();
    out.sort_by(|a, b| b.uses.cmp(&a.uses).then_with(|| a.tag.cmp(&b.tag)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translators::import::xunity::{Dictionaries, Entry};

    #[test]
    fn finds_the_tags_of_a_text() {
        assert_eq!(
            tags_of(
                "Deals <@ba.kw>Arts</> damage, <$ba.stun><@ba.kw>Stuns</></> and <@ba.kw>more</> <i>x</i> <Roadblock>"
            ),
            vec!["@ba.kw", "$ba.stun"]
        );
        assert!(tags_of("plain {atk:0%}").is_empty());
    }

    #[test]
    fn lists_styles_and_terms_with_the_pack_s_russian() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gamedata_const.json"),
            r##"{"RichTextStyles":[{"key":"ba.vup","value":"<color=#0098DC>{0}</color>"},{"key":"ba.odd","value":"<b>{0}</b>"}],
            "TermDescriptionDict":[{"key":"ba.stun","value":{"TermId":"ba.stun","TermName":"Stun","Description":"Unable to act"}}]}"##,
        )
        .unwrap();
        let mut dict = Dictionaries::default();
        for (k, v) in [
            ("Stun", "Оглушение"),
            ("Unable to act", "Не может действовать"),
        ] {
            dict.entries.insert(
                k.to_owned(),
                Entry {
                    value: v.to_owned(),
                    file: "11_ui.txt".to_owned(),
                },
            );
        }
        let matcher = Matcher::new(&dict, None);
        let row = |en: &str| TextRow {
            edit_key: en.to_owned(),
            en: en.to_owned(),
            ru: None,
            file: None,
            plugin_role: None,
            ru_plugin: None,
            sections: Vec::new(),
            places: 0,
        };
        let rows = vec![
            row("ATK <@ba.vup>+10</>, <$ba.stun>Stuns</>"),
            row("DEF <@ba.vup>+5</> <@ev.new>now</>"),
        ];
        let tags = collect(dir.path(), &matcher, &rows);
        let by: HashMap<&str, &GameTag> = tags.iter().map(|t| (t.tag.as_str(), t)).collect();
        assert_eq!(tags[0].tag, "@ba.vup");
        assert_eq!(by["@ba.vup"].uses, 2);
        assert_eq!(by["@ba.vup"].color.as_deref(), Some("#0098DC"));
        assert_eq!(by["@ba.odd"].color, None);
        assert_eq!(by["@ba.odd"].uses, 0);
        let stun = by["$ba.stun"];
        assert_eq!((stun.kind.as_str(), stun.uses), ("term", 1));
        assert_eq!(stun.name_ru.as_deref(), Some("Оглушение"));
        assert_eq!(stun.description_ru.as_deref(), Some("Не может действовать"));
        // Used and defined nowhere: listed all the same.
        assert_eq!(by["@ev.new"].kind, "style");
        assert_eq!(by["@ev.new"].uses, 1);
    }
}
