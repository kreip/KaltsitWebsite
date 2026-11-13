//! Dictionary entries with a skill level's numbers put in ("dealing 230% ATK
//! … Stunning them for 1.25 seconds", one entry per level) tied to the
//! skill's description template of the game data ("dealing
//! <@ba.vup>{atk_scale:0%}</> ATK … for <@ba.vup>{stun}</> seconds"): the
//! import links them (`Linker`), the card lists them under the description,
//! and "Apply to the levels" puts the numbers of each level into a proposal
//! made for the template (`apply`).
//!
//! Two texts match by shape: tags dropped, every substitution and every
//! number a `#`. The level is told by the numbers the blackboard of each
//! level gives (`{atk_scale:0%}` → 230 %); when no level's numbers fit, the
//! entry is tied to the skill with the level unknown — unless the shape fits
//! more than [`MAX_SKILLS`] skills, when it stays untied.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use super::gamedata::{self, GamePlace, OPERATORS};

/// The field of a level's entry at the operator's card.
pub const FIELD: &str = "skill.level";

/// A shape fitting more skills than this ("ATK +#%") ties an entry to none.
const MAX_SKILLS: usize = 3;

static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^<>]*>").expect("the tag pattern"));
/// A substitution, or a number as the game writes one in (`35%`, `1.25`); a
/// sign stays text (the template says `+{atk:0%}`, a value may be negative).
static SLOT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{[^{}<>\n]{1,60}\}|[0-9]+(?:\.[0-9]+)?%?").expect("the slot pattern")
});
/// A sign before a slot: dropped from the shape, as a value's sign may differ from the template's.
static SIGNED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[+\-]#").expect("the sign pattern"));
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("the spaces pattern"));

/// One level of one operator's skill, with its description template and blackboard.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillLevel {
    pub object_id: String,
    pub object_en: String,
    /// The skill's number at the operator, from 1.
    pub idx: i16,
    /// `S1`…`S7`, `M1`…`M3`.
    pub note: String,
    pub description: String,
    pub blackboard: Vec<(String, f64)>,
}

/// Every operator's skill levels, as the card lists their descriptions.
pub fn skill_levels(excel_dir: &Path) -> Vec<SkillLevel> {
    let mut out = Vec::new();
    let Some(characters) = gamedata::read(excel_dir, "character_table") else {
        return out;
    };
    let skills = gamedata::read(excel_dir, "skill_table").unwrap_or(Value::Null);
    let skills = gamedata::index(&skills, "Skills");
    for (id, c) in gamedata::entries(&characters, "Characters") {
        let Some(name) = gamedata::text(c, "Name") else {
            continue;
        };
        for (i, slot) in gamedata::list(c, "Skills").enumerate() {
            let Some(skill) = gamedata::text(slot, "SkillId").and_then(|s| skills.get(s)) else {
                continue;
            };
            for (level_no, level) in gamedata::list(skill, "Levels").enumerate() {
                let Some(description) = gamedata::text(level, "Description") else {
                    continue;
                };
                let blackboard = gamedata::list(level, "Blackboard")
                    .filter_map(|b| {
                        Some((
                            gamedata::text(b, "key")?.to_owned(),
                            b.get("value")?.as_f64()?,
                        ))
                    })
                    .collect();
                out.push(SkillLevel {
                    object_id: id.to_owned(),
                    object_en: name.to_owned(),
                    idx: gamedata::number(i),
                    note: gamedata::skill_level_note(level_no),
                    description: description.to_owned(),
                    blackboard,
                });
            }
        }
    }
    out
}

/// The text without tags, every substitution and number a `#`, spaces collapsed.
pub fn shape(text: &str) -> String {
    let plain = TAG.replace_all(text, "");
    let slots = SLOT.replace_all(&plain, "#");
    let unsigned = SIGNED.replace_all(&slots, "#");
    SPACES.replace_all(unsigned.trim(), " ").into_owned()
}

/// The substitutions and numbers of a text, in order, tags dropped.
fn slots(text: &str) -> Vec<String> {
    let plain = TAG.replace_all(text, "");
    SLOT.find_iter(&plain)
        .map(|m| m.as_str().to_owned())
        .collect()
}

/// A number without its sign (a value's `-30%` against the entry's `30%` after its own `-`).
fn bare(n: &str) -> &str {
    n.trim_start_matches(['+', '-'])
}

/// A value as the game writes it for a substitution's format: `0%` → `35%`,
/// `0.0%` → `35.5%`, `0` → `6`, `0.0` → `1.3`, none → `6` or `1.25`.
pub fn format_value(value: f64, spec: Option<&str>) -> String {
    match spec {
        Some(spec) if spec.ends_with('%') => {
            let decimals = spec.find('.').map_or(0, |d| spec.len() - d - 2);
            format!("{:.*}%", decimals, value * 100.0)
        }
        Some(spec) => {
            let decimals = spec.find('.').map_or(0, |d| spec.len() - d - 1);
            format!("{:.*}", decimals, value)
        }
        None => {
            if value.fract() == 0.0 {
                format!("{}", value as i64)
            } else {
                let s = format!("{value:.6}");
                s.trim_end_matches('0').trim_end_matches('.').to_owned()
            }
        }
    }
}

/// What a substitution of the template gives at a level: `{atk_scale:0%}`
/// → the blackboard's `atk_scale` as `230%`; `{-cost}` is the negated value.
fn value_of(token: &str, blackboard: &[(String, f64)]) -> Option<String> {
    let inner = token.strip_prefix('{')?.strip_suffix('}')?;
    let (name, spec) = match inner.split_once(':') {
        Some((n, s)) => (n, Some(s)),
        None => (inner, None),
    };
    let (name, negate) = match name.strip_prefix('-') {
        Some(n) => (n, true),
        None => (name, false),
    };
    let value = blackboard
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| if negate { -v } else { *v })?;
    Some(format_value(value, spec))
}

/// The numbers a level's entry should hold, slot by slot of the template
/// (a literal number of the template stays), or None when the blackboard
/// lacks a substitution.
fn expected(level: &SkillLevel) -> Option<Vec<String>> {
    slots(&level.description)
        .into_iter()
        .map(|s| {
            if s.starts_with('{') {
                value_of(&s, &level.blackboard)
            } else {
                Some(s)
            }
        })
        .collect()
}

/// The template's slots paired with the entry's numbers, when the two
/// texts have one shape and every literal number of the template is there.
pub fn align(template: &str, entry: &str) -> Option<Vec<(String, String)>> {
    if shape(template) != shape(entry) {
        return None;
    }
    let pairs: Vec<(String, String)> = slots(template).into_iter().zip(slots(entry)).collect();
    pairs
        .iter()
        .all(|(t, e)| t.starts_with('{') || bare(t) == bare(e))
        .then_some(pairs)
}

/// The skill levels indexed by the shape of their description.
pub struct Linker {
    levels: Vec<SkillLevel>,
    by_shape: HashMap<String, Vec<usize>>,
}

/// Where a dictionary entry with numbers belongs: a skill's description
/// template (its text) and the place of the entry at the operator's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelLink {
    pub template: String,
    pub place: GamePlace,
}

impl Linker {
    pub fn new(levels: Vec<SkillLevel>) -> Self {
        let mut by_shape: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, l) in levels.iter().enumerate() {
            by_shape.entry(shape(&l.description)).or_default().push(i);
        }
        Self { levels, by_shape }
    }

    /// The links of one entry: one per skill, its note the levels whose
    /// numbers the entry holds (`S3,S4`), empty when no level's fit.
    pub fn link(&self, entry: &str) -> Vec<LevelLink> {
        if !entry.chars().any(|c| c.is_ascii_digit()) {
            return Vec::new();
        }
        let Some(candidates) = self.by_shape.get(&shape(entry)) else {
            return Vec::new();
        };
        let numbers: Vec<String> = slots(entry).iter().map(|s| bare(s).to_owned()).collect();
        // Per skill (object, idx): a level of it (one whose numbers fit when
        // there is one: its description is the template) and the fitting levels.
        let mut skills: Vec<(usize, Vec<&str>)> = Vec::new();
        for &i in candidates {
            let l = &self.levels[i];
            let exact = expected(l).is_some_and(|e| {
                e.len() == numbers.len()
                    && e.iter()
                        .map(|n| bare(n))
                        .eq(numbers.iter().map(String::as_str))
            });
            let same_skill =
                |j: usize| self.levels[j].object_id == l.object_id && self.levels[j].idx == l.idx;
            match skills.iter_mut().find(|(j, _)| same_skill(*j)) {
                Some((j, notes)) => {
                    if exact {
                        if notes.is_empty() {
                            *j = i;
                        }
                        notes.push(&l.note);
                    }
                }
                None => skills.push((i, if exact { vec![&l.note] } else { Vec::new() })),
            }
        }
        let any_exact = skills.iter().any(|(_, notes)| !notes.is_empty());
        if any_exact {
            skills.retain(|(_, notes)| !notes.is_empty());
        } else if skills.len() > MAX_SKILLS {
            return Vec::new();
        }
        skills
            .into_iter()
            .map(|(i, notes)| {
                let l = &self.levels[i];
                LevelLink {
                    template: l.description.clone(),
                    place: GamePlace {
                        section: OPERATORS,
                        object_id: l.object_id.clone(),
                        object_en: l.object_en.clone(),
                        field: FIELD,
                        idx: l.idx,
                        note: notes.join(","),
                        text: entry.to_owned(),
                        of_key: None,
                    },
                }
            })
            .collect()
    }
}

/// A proposal made for the template, with the entry's numbers put into
/// its substitutions; the tags dropped when the entry has none (the
/// dictionary keeps a level's text plain). None when the Russian holds a
/// substitution the template lacks, or the entry does not fit the template.
pub fn apply(template: &str, entry: &str, russian: &str) -> Option<String> {
    let pairs = align(template, entry)?;
    let values: HashMap<&str, &str> = pairs
        .iter()
        .filter(|(t, _)| t.starts_with('{'))
        .map(|(t, e)| (t.as_str(), e.as_str()))
        .collect();
    let mut out = String::with_capacity(russian.len());
    let mut last = 0;
    let mut failed = false;
    for m in SLOT.find_iter(russian) {
        if !m.as_str().starts_with('{') {
            continue;
        }
        out.push_str(&russian[last..m.start()]);
        last = m.end();
        match values.get(m.as_str()) {
            Some(v) => out.push_str(v),
            None => {
                failed = true;
                break;
            }
        }
    }
    if failed {
        return None;
    }
    out.push_str(&russian[last..]);
    if !TAG.is_match(entry) {
        out = TAG.replace_all(&out, "").into_owned();
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(note: &str, description: &str, blackboard: &[(&str, f64)]) -> SkillLevel {
        SkillLevel {
            object_id: "char_010_chen".into(),
            object_en: "Ch'en".into(),
            idx: 1,
            note: note.into(),
            description: description.into(),
            blackboard: blackboard
                .iter()
                .map(|(k, v)| ((*k).to_owned(), *v))
                .collect(),
        }
    }

    const CHEN: &str = "On the next attack, strike the enemy using the sheath, dealing <@ba.vup>{atk_scale:0%}</> ATK as Physical damage and <$ba.stun>Stunning</> them for <@ba.vup>{stun}</> seconds";

    #[test]
    fn values_are_written_as_the_game_writes_them() {
        assert_eq!(format_value(2.3, Some("0%")), "230%");
        assert_eq!(format_value(0.355, Some("0.0%")), "35.5%");
        assert_eq!(format_value(1.25, None), "1.25");
        assert_eq!(format_value(1.0, None), "1");
        assert_eq!(format_value(6.0, Some("0")), "6");
        assert_eq!(format_value(1.3, Some("0.0")), "1.3");
        assert_eq!(
            value_of("{-cost}", &[("cost".into(), 6.0)]).as_deref(),
            Some("-6")
        );
    }

    #[test]
    fn an_entry_is_tied_to_its_skill_and_level() {
        let linker = Linker::new(vec![
            level("S1", CHEN, &[("atk_scale", 2.0), ("stun", 1.0)]),
            level("S4", CHEN, &[("atk_scale", 2.3), ("stun", 1.25)]),
            level("S5", CHEN, &[("atk_scale", 2.4), ("stun", 1.25)]),
        ]);
        let links = linker.link("On the next attack, strike the enemy using the sheath, dealing 230% ATK as Physical damage and Stunning them for 1.25 seconds");
        assert_eq!(links.len(), 1);
        assert_eq!(
            (
                links[0].place.field,
                links[0].place.note.as_str(),
                links[0].place.idx
            ),
            (FIELD, "S4", 1)
        );
        assert_eq!(links[0].template, CHEN);
        // The shape fits, no level's numbers do: tied to the skill, the level unknown.
        let links = linker.link("On the next attack, strike the enemy using the sheath, dealing 999% ATK as Physical damage and Stunning them for 9 seconds");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].place.note, "");
        // Another text: nothing.
        assert!(linker.link("Deals 230% ATK").is_empty());
        assert!(linker.link("No numbers here").is_empty());
    }

    #[test]
    fn a_shape_fitting_too_many_skills_ties_nothing() {
        let mut levels = Vec::new();
        for i in 0..4 {
            let mut l = level("S1", "DEF <@ba.vup>+{def:0%}</>", &[("def", 0.1)]);
            l.object_id = format!("char_{i}");
            levels.push(l);
        }
        let linker = Linker::new(levels);
        assert!(linker.link("DEF +99%").is_empty());
        // The numbers fit: every skill they fit.
        assert_eq!(linker.link("DEF +10%").len(), 4);
    }

    #[test]
    fn the_numbers_go_into_the_russian() {
        let entry = "On the next attack, strike the enemy using the sheath, dealing 230% ATK as Physical damage and Stunning them for 1.25 seconds";
        let russian = "Следующей атакой бьёт врага ножнами, нанося <@ba.vup>{atk_scale:0%}</> АТК физического урона и <$ba.stun>Оглушая</> его на <@ba.vup>{stun}</> секунд";
        assert_eq!(
            apply(CHEN, entry, russian).as_deref(),
            Some(
                "Следующей атакой бьёт врага ножнами, нанося 230% АТК физического урона и Оглушая его на 1.25 секунд"
            )
        );
        // A substitution the template lacks: not sure.
        assert_eq!(apply(CHEN, entry, "Урон {dmg} и {stun}"), None);
        // The entry keeps its tags: so does the result.
        assert_eq!(
            apply(
                "ATK <@ba.vup>+{atk:0%}</>",
                "ATK <@ba.vup>+35%</>",
                "АТК <@ba.vup>+{atk:0%}</>"
            )
            .as_deref(),
            Some("АТК <@ba.vup>+35%</>")
        );
    }
}
