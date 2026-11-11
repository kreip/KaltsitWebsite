//! Where the game's own data shows a text: the operators (names, traits,
//! talents, skills, modules, base skills, outfits), their archives (files,
//! voice lines, module and outfit stories), the enemies, the stages and
//! zones, the items. Read from the EN client's `gamedata/excel` tables as
//! plain JSON, so a table the site's typed game data does not model still
//! reaches the section. A table that is missing or shaped differently only
//! leaves its places out.
//!
//! The tables keep a dictionary as a list of `{ "key": …, "value": … }`.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// The sections a game text belongs to, in the order the section shows them.
pub const OPERATORS: &str = "operators";
pub const ENEMIES: &str = "enemies";
pub const STAGES: &str = "stages";
pub const ITEMS: &str = "items";
pub const ARCHIVES: &str = "archives";
/// Everything else: the other tables' texts and dictionary entries tied to nothing known.
pub const OTHER: &str = "other";
pub const STORY: &str = "story";

/// One text at one place of one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamePlace {
    pub section: &'static str,
    /// The object's id in the game data (`char_002_amiya`, `enemy_1007_slime`, `main_00-01`).
    pub object_id: String,
    /// The object's English name.
    pub object_en: String,
    /// What the text is: `name`, `trait`, `talent.name`, `skill.description`, `module.name`, ...
    pub field: &'static str,
    /// Which one of several (skill 2, talent 1, file 3), from 1; 0 when there is one.
    pub idx: i16,
    pub text: String,
}

fn read(dir: &Path, table: &str) -> Option<Value> {
    let bytes = std::fs::read(dir.join(format!("{table}.json"))).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// The entries of a dictionary kept as a list of `{ key, value }` (string keys only).
fn entries<'a>(table: &'a Value, name: &str) -> impl Iterator<Item = (&'a str, &'a Value)> {
    table
        .get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| Some((e.get("key")?.as_str()?, e.get("value")?)))
}

fn index<'a>(table: &'a Value, name: &str) -> HashMap<&'a str, &'a Value> {
    entries(table, name).collect()
}

/// A field's text, when it has any.
fn text<'a>(v: &'a Value, name: &str) -> Option<&'a str> {
    v.get(name)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

fn list<'a>(v: &'a Value, name: &str) -> impl Iterator<Item = &'a Value> {
    v.get(name).and_then(Value::as_array).into_iter().flatten()
}

fn number(i: usize) -> i16 {
    i16::try_from(i + 1).unwrap_or(i16::MAX)
}

/// Collects the places of one object.
struct Object<'o> {
    out: &'o mut Vec<GamePlace>,
    id: &'o str,
    name: &'o str,
}

impl Object<'_> {
    fn add(&mut self, section: &'static str, field: &'static str, idx: i16, text: Option<&str>) {
        let Some(text) = text.filter(|t| !t.trim().is_empty()) else {
            return;
        };
        self.out.push(GamePlace {
            section,
            object_id: self.id.to_owned(),
            object_en: self.name.to_owned(),
            field,
            idx,
            text: text.to_owned(),
        });
    }
}

/// Every string under `v` kept in a field named one of `names`.
fn deep<'a>(v: &'a Value, names: &[&str], out: &mut Vec<&'a str>) {
    match v {
        Value::Object(map) => {
            for (k, x) in map {
                match x {
                    Value::String(s) if names.contains(&k.as_str()) => out.push(s),
                    _ => deep(x, names, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|x| deep(x, names, out)),
        _ => {}
    }
}

fn operators(dir: &Path, out: &mut Vec<GamePlace>) {
    let Some(characters) = read(dir, "character_table") else {
        return;
    };
    let skills = read(dir, "skill_table").unwrap_or(Value::Null);
    let skills = index(&skills, "Skills");
    let equip = read(dir, "uniequip_table").unwrap_or(Value::Null);
    let subclasses = index(&equip, "SubProfDict");
    let missions = index(&equip, "MissionList");
    let mut modules: HashMap<&str, Vec<&Value>> = HashMap::new();
    for (_, e) in entries(&equip, "EquipDict") {
        if let Some(c) = text(e, "CharId") {
            modules.entry(c).or_default().push(e);
        }
    }
    let battle = read(dir, "battle_equip_table").unwrap_or(Value::Null);
    let battle = index(&battle, "Equips");
    let building = read(dir, "building_data").unwrap_or(Value::Null);
    let base_chars = index(&building, "Chars");
    let buffs = index(&building, "Buffs");
    let skin_table = read(dir, "skin_table").unwrap_or(Value::Null);
    let mut skins: HashMap<&str, Vec<&Value>> = HashMap::new();
    for (_, s) in entries(&skin_table, "CharSkins") {
        if let Some(c) = text(s, "CharId") {
            skins.entry(c).or_default().push(s);
        }
    }
    let handbook = read(dir, "handbook_info_table").unwrap_or(Value::Null);
    let files = index(&handbook, "HandbookDict");
    let words_table = read(dir, "charword_table").unwrap_or(Value::Null);
    let mut words: HashMap<&str, Vec<&Value>> = HashMap::new();
    for (_, w) in entries(&words_table, "CharWords") {
        if let Some(c) = text(w, "CharId") {
            words.entry(c).or_default().push(w);
        }
    }

    for (id, c) in entries(&characters, "Characters") {
        let Some(name) = text(c, "Name") else {
            continue;
        };
        let mut o = Object { out, id, name };
        o.add(OPERATORS, "name", 0, Some(name));
        o.add(OPERATORS, "appellation", 0, text(c, "Appellation"));
        if let Some(sub) = text(c, "SubProfessionId").and_then(|s| subclasses.get(s)) {
            o.add(OPERATORS, "subclass", 0, text(sub, "SubProfessionName"));
        }
        for tag in list(c, "TagList") {
            o.add(OPERATORS, "tag", 0, tag.as_str());
        }
        o.add(OPERATORS, "trait", 0, text(c, "Description"));
        if let Some(tr) = c.get("Trait") {
            let mut found = Vec::new();
            deep(
                tr,
                &["OverrideDescripton", "AdditionalDescription"],
                &mut found,
            );
            for t in found {
                o.add(OPERATORS, "trait", 0, Some(t));
            }
        }
        for (i, talent) in list(c, "Talents").enumerate() {
            for cand in list(talent, "Candidates") {
                o.add(OPERATORS, "talent.name", number(i), text(cand, "Name"));
                o.add(
                    OPERATORS,
                    "talent.description",
                    number(i),
                    text(cand, "Description"),
                );
            }
        }
        for (i, rank) in list(c, "PotentialRanks").enumerate() {
            o.add(OPERATORS, "potential", number(i), text(rank, "Description"));
        }
        for (i, slot) in list(c, "Skills").enumerate() {
            let Some(skill) = text(slot, "SkillId").and_then(|s| skills.get(s)) else {
                continue;
            };
            for level in list(skill, "Levels") {
                o.add(OPERATORS, "skill.name", number(i), text(level, "Name"));
                o.add(
                    OPERATORS,
                    "skill.description",
                    number(i),
                    text(level, "Description"),
                );
            }
        }
        let mut own = modules.get(id).cloned().unwrap_or_default();
        own.sort_by_key(|m| m.get("CharEquipOrder").and_then(Value::as_i64).unwrap_or(0));
        for (i, m) in own.iter().enumerate() {
            let n = number(i);
            o.add(OPERATORS, "module.name", n, text(m, "UniEquipName"));
            o.add(ARCHIVES, "module.story", n, text(m, "UniEquipDesc"));
            for mission in list(m, "MissionList") {
                if let Some(mission) = mission.as_str().and_then(|k| missions.get(k)) {
                    o.add(OPERATORS, "module.mission", n, text(mission, "Desc"));
                }
            }
            if let Some(b) = text(m, "UniEquipId").and_then(|k| battle.get(k)) {
                let mut found = Vec::new();
                deep(
                    b,
                    &[
                        "UpgradeDescription",
                        "AdditionalDescription",
                        "OverrideDescripton",
                    ],
                    &mut found,
                );
                for t in found {
                    o.add(OPERATORS, "module.effect", n, Some(t));
                }
            }
        }
        if let Some(base) = base_chars.get(id) {
            for (i, slot) in list(base, "BuffChar").enumerate() {
                for data in list(slot, "BuffData") {
                    if let Some(buff) = text(data, "BuffId").and_then(|k| buffs.get(k)) {
                        o.add(OPERATORS, "base.name", number(i), text(buff, "BuffName"));
                        o.add(
                            OPERATORS,
                            "base.description",
                            number(i),
                            text(buff, "Description"),
                        );
                    }
                }
            }
        }
        for (i, skin) in skins.get(id).into_iter().flatten().enumerate() {
            let Some(d) = skin.get("DisplaySkin") else {
                continue;
            };
            // The default outfits have no name of their own.
            let Some(skin_name) = text(d, "SkinName") else {
                continue;
            };
            let n = number(i);
            o.add(OPERATORS, "skin.name", n, Some(skin_name));
            o.add(OPERATORS, "skin.group", n, text(d, "SkinGroupName"));
            o.add(OPERATORS, "skin.usage", n, text(d, "Usage"));
            o.add(OPERATORS, "skin.description", n, text(d, "Description"));
            o.add(ARCHIVES, "skin.story", n, text(d, "Content"));
            o.add(ARCHIVES, "skin.dialog", n, text(d, "Dialog"));
        }
        if let Some(file) = files.get(id) {
            for (i, part) in list(file, "StoryTextAudio").enumerate() {
                o.add(ARCHIVES, "file.title", number(i), text(part, "StoryTitle"));
                for story in list(part, "Stories") {
                    o.add(ARCHIVES, "file.text", number(i), text(story, "StoryText"));
                }
            }
        }
        let mut lines = words.get(id).cloned().unwrap_or_default();
        lines.sort_by_key(|w| w.get("VoiceIndex").and_then(Value::as_i64).unwrap_or(0));
        for (i, w) in lines.iter().enumerate() {
            o.add(ARCHIVES, "voice.title", number(i), text(w, "VoiceTitle"));
            o.add(ARCHIVES, "voice.text", number(i), text(w, "VoiceText"));
        }
    }
}

fn enemies(dir: &Path, out: &mut Vec<GamePlace>) {
    let Some(table) = read(dir, "enemy_handbook_table") else {
        return;
    };
    for (id, e) in entries(&table, "EnemyData") {
        let Some(name) = text(e, "Name") else {
            continue;
        };
        let mut o = Object { out, id, name };
        o.add(ENEMIES, "name", 0, Some(name));
        o.add(ENEMIES, "description", 0, text(e, "Description"));
        for (i, a) in list(e, "AbilityList").enumerate() {
            o.add(ENEMIES, "ability", number(i), text(a, "Text"));
        }
    }
}

fn stages(dir: &Path, out: &mut Vec<GamePlace>) {
    if let Some(table) = read(dir, "stage_table") {
        for (id, s) in entries(&table, "Stages") {
            let name = text(s, "Name").unwrap_or_default();
            let code = text(s, "Code").unwrap_or_default();
            let label = format!("{code} {name}");
            let label = label.trim();
            if label.is_empty() {
                continue;
            }
            let mut o = Object {
                out,
                id,
                name: label,
            };
            o.add(STAGES, "name", 0, Some(name));
            o.add(STAGES, "description", 0, text(s, "Description"));
        }
    }
    if let Some(table) = read(dir, "zone_table") {
        for (id, z) in entries(&table, "Zones") {
            let label = [text(z, "ZoneNameFirst"), text(z, "ZoneNameSecond")]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ");
            if label.is_empty() {
                continue;
            }
            let mut o = Object {
                out,
                id,
                name: &label,
            };
            for (field, key) in [
                ("zone.first", "ZoneNameFirst"),
                ("zone.second", "ZoneNameSecond"),
                ("zone.third", "ZoneNameThird"),
                ("zone.title", "ZoneNameTitleCurrent"),
                ("zone.title", "ZoneNameTitleUnCurrent"),
                ("zone.title", "ZoneNameTitleEx"),
                ("zone.locked", "LockedText"),
            ] {
                o.add(STAGES, field, 0, text(z, key));
            }
        }
    }
}

fn items(dir: &Path, out: &mut Vec<GamePlace>) {
    let Some(table) = read(dir, "item_table") else {
        return;
    };
    for (id, it) in entries(&table, "Items") {
        let Some(name) = text(it, "Name") else {
            continue;
        };
        let mut o = Object { out, id, name };
        o.add(ITEMS, "name", 0, Some(name));
        o.add(ITEMS, "usage", 0, text(it, "Usage"));
        o.add(ITEMS, "description", 0, text(it, "Description"));
        o.add(ITEMS, "obtain", 0, text(it, "ObtainApproach"));
    }
}

/// Every place of the named sections, in the tables' order; a text repeated
/// at one place of one object (a skill's name on each of its levels) once.
pub fn places(excel_dir: &Path) -> Vec<GamePlace> {
    let mut out = Vec::new();
    operators(excel_dir, &mut out);
    enemies(excel_dir, &mut out);
    stages(excel_dir, &mut out);
    items(excel_dir, &mut out);
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| {
        seen.insert((
            p.section,
            p.object_id.clone(),
            p.field,
            p.idx,
            p.text.clone(),
        ))
    });
    out
}

/// Every string of every table (`table name`, `text`), for the texts no
/// named section holds: the caller keeps those the pack translates.
pub fn every_string(excel_dir: &Path, mut each: impl FnMut(&str, &str)) {
    let Ok(dir) = std::fs::read_dir(excel_dir) else {
        return;
    };
    let mut files: Vec<_> = dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    for path in files {
        let Some(table) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        let Some(json) = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        else {
            continue;
        };
        let mut stack = vec![&json];
        while let Some(v) = stack.pop() {
            match v {
                Value::String(s) if !s.trim().is_empty() => each(&table, s),
                Value::Array(items) => stack.extend(items.iter()),
                Value::Object(map) => stack.extend(map.values()),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, table: &str, json: &str) {
        std::fs::write(dir.join(format!("{table}.json")), json).unwrap();
    }

    #[test]
    fn an_operator_brings_its_skills_modules_base_skills_and_archives() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(
            d,
            "character_table",
            r#"{"Characters":[{"key":"char_1","value":{"Name":"Amiya","Appellation":" ","Description":"Deals Arts damage",
            "SubProfessionId":"corecaster","TagList":["DPS"],
            "Talents":[{"Candidates":[{"Name":"Emotion Absorption","Description":"Recovers SP"},{"Name":"Emotion Absorption","Description":"Recovers more SP"}]}],
            "Skills":[{"SkillId":"sk1"}]}},
            {"key":"char_2","value":{"Name":"","Description":"no name"}}]}"#,
        );
        write(
            d,
            "skill_table",
            r#"{"Skills":[{"key":"sk1","value":{"Levels":[{"Name":"Tactical Chant","Description":"ASPD +{x}"},{"Name":"Tactical Chant","Description":"ASPD +{x}"}]}}]}"#,
        );
        write(
            d,
            "uniequip_table",
            r#"{"SubProfDict":[{"key":"corecaster","value":{"SubProfessionName":"Core Caster"}}],
            "MissionList":[{"key":"m1","value":{"Desc":"Clear 1-1"}}],
            "EquipDict":[{"key":"e1","value":{"CharId":"char_1","CharEquipOrder":1,"UniEquipId":"e1","UniEquipName":"Ring","UniEquipDesc":"A long story","MissionList":["m1"]}}]}"#,
        );
        write(
            d,
            "battle_equip_table",
            r#"{"Equips":[{"key":"e1","value":{"Phases":[{"Parts":[{"X":{"Candidates":[{"UpgradeDescription":"Talent improved"}]}}]}]}}]}"#,
        );
        write(
            d,
            "building_data",
            r#"{"Chars":[{"key":"char_1","value":{"BuffChar":[{"BuffData":[{"BuffId":"b1"}]}]}}],
            "Buffs":[{"key":"b1","value":{"BuffName":"Coordinator","Description":"Order speed +5%"}}]}"#,
        );
        write(
            d,
            "handbook_info_table",
            r#"{"HandbookDict":[{"key":"char_1","value":{"StoryTextAudio":[{"StoryTitle":"Profile","Stories":[{"StoryText":"She leads Rhodes Island."}]}]}}]}"#,
        );
        write(
            d,
            "charword_table",
            r#"{"CharWords":[{"key":"w1","value":{"CharId":"char_1","VoiceIndex":1,"VoiceTitle":"Greeting","VoiceText":"Hello, Doctor."}}]}"#,
        );
        let all = places(d);
        let got: Vec<(&str, &str, i16, &str)> = all
            .iter()
            .map(|p| (p.section, p.field, p.idx, p.text.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (OPERATORS, "name", 0, "Amiya"),
                (OPERATORS, "subclass", 0, "Core Caster"),
                (OPERATORS, "tag", 0, "DPS"),
                (OPERATORS, "trait", 0, "Deals Arts damage"),
                (OPERATORS, "talent.name", 1, "Emotion Absorption"),
                (OPERATORS, "talent.description", 1, "Recovers SP"),
                (OPERATORS, "talent.description", 1, "Recovers more SP"),
                (OPERATORS, "skill.name", 1, "Tactical Chant"),
                (OPERATORS, "skill.description", 1, "ASPD +{x}"),
                (OPERATORS, "module.name", 1, "Ring"),
                (ARCHIVES, "module.story", 1, "A long story"),
                (OPERATORS, "module.mission", 1, "Clear 1-1"),
                (OPERATORS, "module.effect", 1, "Talent improved"),
                (OPERATORS, "base.name", 1, "Coordinator"),
                (OPERATORS, "base.description", 1, "Order speed +5%"),
                (ARCHIVES, "file.title", 1, "Profile"),
                (ARCHIVES, "file.text", 1, "She leads Rhodes Island."),
                (ARCHIVES, "voice.title", 1, "Greeting"),
                (ARCHIVES, "voice.text", 1, "Hello, Doctor."),
            ]
        );
    }

    #[test]
    fn enemies_stages_zones_and_items() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(
            d,
            "enemy_handbook_table",
            r#"{"EnemyData":[{"key":"enemy_1","value":{"Name":"Slug","Description":"Slow.","AbilityList":[{"Text":"Explodes"}]}}]}"#,
        );
        write(
            d,
            "stage_table",
            r#"{"Stages":[{"key":"main_00-01","value":{"Code":"0-1","Name":"Collapse","Description":"Hold the line."}}]}"#,
        );
        write(
            d,
            "zone_table",
            r#"{"Zones":[{"key":"main_0","value":{"ZoneNameFirst":"Prologue","ZoneNameSecond":"Evil Time"}}]}"#,
        );
        write(
            d,
            "item_table",
            r#"{"Items":[{"key":"4001","value":{"Name":"LMD","Usage":"Currency.","Description":"Money.","ObtainApproach":""}}]}"#,
        );
        let got = places(d);
        let at = |section: &str| -> Vec<(String, &str, String)> {
            got.iter()
                .filter(|p| p.section == section)
                .map(|p| (p.object_en.clone(), p.field, p.text.clone()))
                .collect()
        };
        assert_eq!(
            at(ENEMIES),
            vec![
                ("Slug".into(), "name", "Slug".into()),
                ("Slug".into(), "description", "Slow.".into()),
                ("Slug".into(), "ability", "Explodes".into()),
            ]
        );
        assert_eq!(
            at(STAGES),
            vec![
                ("0-1 Collapse".into(), "name", "Collapse".into()),
                (
                    "0-1 Collapse".into(),
                    "description",
                    "Hold the line.".into()
                ),
                ("Prologue Evil Time".into(), "zone.first", "Prologue".into()),
                (
                    "Prologue Evil Time".into(),
                    "zone.second",
                    "Evil Time".into()
                ),
            ]
        );
        assert_eq!(at(ITEMS).len(), 3);
        let mut all = Vec::new();
        every_string(d, |table, s| all.push((table.to_owned(), s.to_owned())));
        assert!(all.contains(&("item_table".to_owned(), "LMD".to_owned())));
        assert!(all.contains(&("zone_table".to_owned(), "main_0".to_owned())));
    }
}
