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
    /// What tells this text from the others at the same field (a talent at
    /// elite 2 and with potential 6, a skill's description at mastery 3, a
    /// stage's challenge mode), as codes: variants joined by `,`, the parts
    /// of one by `+`. `E2` elite, `L30` operator level, `P6` potential, `S7`
    /// skill level, `M3` mastery, `T2` module stage, `N1` the number in a
    /// list, `C` challenge mode, `DE` / `DN` / `DT` the story / standard /
    /// adverse environment, `#…` a code of the game as it is. Empty when
    /// nothing is known.
    pub note: String,
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
        self.noted(section, field, idx, "", text);
    }

    /// A text with what tells it from the others at its field (`GamePlace::note`).
    fn noted(
        &mut self,
        section: &'static str,
        field: &'static str,
        idx: i16,
        note: &str,
        text: Option<&str>,
    ) {
        let Some(text) = text.filter(|t| !t.trim().is_empty()) else {
            return;
        };
        self.out.push(GamePlace {
            section,
            object_id: self.id.to_owned(),
            object_en: self.name.to_owned(),
            field,
            idx,
            note: note.to_owned(),
            text: text.to_owned(),
        });
    }
}

/// When a talent, a base skill or a module's effect is there: `E2`, `E1+L30`,
/// `E2+P6` (the elite phase, the operator's level above 1, the potential).
fn unlock_note(v: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    let cond = v.get("UnlockCondition").or_else(|| v.get("Cond"));
    if let Some(cond) = cond {
        if let Some(phase) = text(cond, "Phase").and_then(|p| p.strip_prefix("PHASE_")) {
            parts.push(format!("E{phase}"));
        }
        if let Some(level) = cond.get("Level").and_then(Value::as_i64).filter(|l| *l > 1) {
            parts.push(format!("L{level}"));
        }
    }
    if let Some(rank) = v
        .get("RequiredPotentialRank")
        .and_then(Value::as_i64)
        .filter(|r| *r > 0)
    {
        parts.push(format!("P{}", rank + 1));
    }
    parts.join("+")
}

/// A skill's level in the list of its levels: 1 to 7, then mastery 1 to 3.
fn skill_level_note(i: usize) -> String {
    if i < 7 {
        format!("S{}", i + 1)
    } else {
        format!("M{}", i - 6)
    }
}

/// Every text of a module's effects with the stage it comes at and its
/// unlock condition (`T2+E2+P6`).
fn module_effects<'a>(equip: &'a Value, out: &mut Vec<(String, &'a str)>) {
    const NAMES: [&str; 3] = [
        "UpgradeDescription",
        "AdditionalDescription",
        "OverrideDescripton",
    ];
    for phase in list(equip, "Phases") {
        let stage = phase
            .get("EquipLevel")
            .and_then(Value::as_i64)
            .map(|l| format!("T{l}"));
        let mut found: Vec<(String, &str)> = Vec::new();
        candidates(phase, &NAMES, &mut found);
        for (unlock, t) in found {
            // The stage says when; the potential is what differs inside a stage.
            let potential = unlock.split('+').find(|p| p.starts_with('P'));
            let note = [stage.as_deref(), potential]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("+");
            out.push((note, t));
        }
    }
}

/// Every string under `v` kept in a field named one of `names`, with the
/// unlock note of the object holding it.
fn candidates<'a>(v: &'a Value, names: &[&str], out: &mut Vec<(String, &'a str)>) {
    match v {
        Value::Object(map) => {
            for (k, x) in map {
                match x {
                    Value::String(s) if names.contains(&k.as_str()) => {
                        out.push((unlock_note(v), s));
                    }
                    _ => candidates(x, names, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|x| candidates(x, names, out)),
        _ => {}
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

/// The class as the game names it: the table keeps a code (`WARRIOR`), the
/// name stands in the interface. Summons and devices have none.
fn class_name(profession: &str) -> Option<&'static str> {
    Some(match profession {
        "PIONEER" => "Vanguard",
        "WARRIOR" => "Guard",
        "TANK" => "Defender",
        "SNIPER" => "Sniper",
        "CASTER" => "Caster",
        "MEDIC" => "Medic",
        "SUPPORT" => "Supporter",
        "SPECIAL" => "Specialist",
        _ => return None,
    })
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
        o.add(
            OPERATORS,
            "class",
            0,
            text(c, "Profession").and_then(class_name),
        );
        if let Some(sub) = text(c, "SubProfessionId").and_then(|s| subclasses.get(s)) {
            o.add(OPERATORS, "subclass", 0, text(sub, "SubProfessionName"));
        }
        for (i, tag) in list(c, "TagList").enumerate() {
            o.noted(OPERATORS, "tag", 0, &format!("N{}", i + 1), tag.as_str());
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
                let note = unlock_note(cand);
                o.noted(
                    OPERATORS,
                    "talent.name",
                    number(i),
                    &note,
                    text(cand, "Name"),
                );
                o.noted(
                    OPERATORS,
                    "talent.description",
                    number(i),
                    &note,
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
            for (level_no, level) in list(skill, "Levels").enumerate() {
                let note = skill_level_note(level_no);
                o.noted(
                    OPERATORS,
                    "skill.name",
                    number(i),
                    &note,
                    text(level, "Name"),
                );
                o.noted(
                    OPERATORS,
                    "skill.description",
                    number(i),
                    &note,
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
            for (k, mission) in list(m, "MissionList").enumerate() {
                if let Some(mission) = mission.as_str().and_then(|k| missions.get(k)) {
                    o.noted(
                        OPERATORS,
                        "module.mission",
                        n,
                        &format!("N{}", k + 1),
                        text(mission, "Desc"),
                    );
                }
            }
            if let Some(b) = text(m, "UniEquipId").and_then(|k| battle.get(k)) {
                let mut found = Vec::new();
                module_effects(b, &mut found);
                for (note, t) in found {
                    o.noted(OPERATORS, "module.effect", n, &note, Some(t));
                }
            }
        }
        if let Some(base) = base_chars.get(id) {
            for (i, slot) in list(base, "BuffChar").enumerate() {
                for data in list(slot, "BuffData") {
                    if let Some(buff) = text(data, "BuffId").and_then(|k| buffs.get(k)) {
                        let note = unlock_note(data);
                        o.noted(
                            OPERATORS,
                            "base.name",
                            number(i),
                            &note,
                            text(buff, "BuffName"),
                        );
                        o.noted(
                            OPERATORS,
                            "base.description",
                            number(i),
                            &note,
                            text(buff, "Description"),
                        );
                    }
                }
            }
        }
        // The default outfits have no name of their own: the named ones are numbered from 1.
        let named = skins.get(id).into_iter().flatten().filter_map(|skin| {
            let d = skin.get("DisplaySkin")?;
            Some((d, text(d, "SkinName")?))
        });
        for (i, (d, skin_name)) in named.enumerate() {
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
        // Enemies under one name are one card: the handbook's code tells their texts apart.
        let note = text(e, "EnemyIndex")
            .map(|code| format!("#{code}"))
            .unwrap_or_default();
        let mut o = Object { out, id, name };
        o.noted(ENEMIES, "name", 0, &note, Some(name));
        o.noted(ENEMIES, "description", 0, &note, text(e, "Description"));
        for (i, a) in list(e, "AbilityList").enumerate() {
            o.noted(ENEMIES, "ability", number(i), &note, text(a, "Text"));
        }
    }
}

/// Which of the stages under one code and name this is: the challenge mode
/// (`C`), the environment of a later main chapter (`DE` story, `DN`
/// standard, `DT` adverse); a plain stage has no note.
fn stage_note(s: &Value) -> String {
    let mut parts: Vec<&str> = Vec::new();
    match text(s, "DiffGroup") {
        Some("EASY") => parts.push("DE"),
        Some("NORMAL") => parts.push("DN"),
        Some("TOUGH") => parts.push("DT"),
        _ => {}
    }
    match text(s, "Difficulty") {
        Some("FOUR_STAR" | "SIX_STAR") => parts.push("C"),
        _ => {}
    }
    parts.join("+")
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
            let note = stage_note(s);
            o.noted(STAGES, "name", 0, &note, Some(name));
            o.noted(STAGES, "description", 0, &note, text(s, "Description"));
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
            for (field, key, note) in [
                ("zone.first", "ZoneNameFirst", ""),
                ("zone.second", "ZoneNameSecond", ""),
                ("zone.third", "ZoneNameThird", ""),
                ("zone.title", "ZoneNameTitleCurrent", "N1"),
                ("zone.title", "ZoneNameTitleUnCurrent", "N2"),
                ("zone.title", "ZoneNameTitleEx", "N3"),
                ("zone.locked", "LockedText", ""),
            ] {
                o.noted(STAGES, field, 0, note, text(z, key));
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
        // Items under one name are one card: the item's id tells their texts apart.
        let note = format!("#{id}");
        let mut o = Object { out, id, name };
        o.noted(ITEMS, "name", 0, &note, Some(name));
        o.noted(ITEMS, "usage", 0, &note, text(it, "Usage"));
        o.noted(ITEMS, "description", 0, &note, text(it, "Description"));
        o.noted(ITEMS, "obtain", 0, &note, text(it, "ObtainApproach"));
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
    // The first of the same text at the same place stays, with the notes of them all.
    let mut first: HashMap<(&'static str, String, &'static str, i16, String), usize> =
        HashMap::new();
    let mut kept: Vec<GamePlace> = Vec::with_capacity(out.len());
    for p in out {
        let key = (
            p.section,
            p.object_id.clone(),
            p.field,
            p.idx,
            p.text.clone(),
        );
        match first.get(&key) {
            Some(&i) => {
                let note = &mut kept[i].note;
                if !p.note.is_empty() && !note.split(',').any(|n| n == p.note) {
                    if !note.is_empty() {
                        note.push(',');
                    }
                    note.push_str(&p.note);
                }
            }
            None => {
                first.insert(key, kept.len());
                kept.push(p);
            }
        }
    }
    kept
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
            "Profession":"CASTER","SubProfessionId":"corecaster","TagList":["DPS"],
            "Talents":[{"Candidates":[{"Name":"Emotion Absorption","Description":"Recovers SP"},{"Name":"Emotion Absorption","Description":"Recovers more SP"}]}],
            "Skills":[{"SkillId":"sk1"}]}},
            {"key":"char_2","value":{"Name":"","Description":"no name"}},
            {"key":"trap_1","value":{"Name":"Crate","Profession":"TRAP"}}]}"#,
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
            "skin_table",
            r#"{"CharSkins":[{"key":"s0","value":{"CharId":"char_1","DisplaySkin":{"SkinName":null,"Content":"Default."}}},
            {"key":"s1","value":{"CharId":"char_1","DisplaySkin":{"SkinName":"Newsgirl","SkinGroupName":"Test Collection","Usage":"A favorite outfit.","Content":"Gray sky."}}}]}"#,
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
                (OPERATORS, "class", 0, "Caster"),
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
                // The outfit after the default one is the first with a name.
                (OPERATORS, "skin.name", 1, "Newsgirl"),
                (OPERATORS, "skin.group", 1, "Test Collection"),
                (OPERATORS, "skin.usage", 1, "A favorite outfit."),
                (ARCHIVES, "skin.story", 1, "Gray sky."),
                (ARCHIVES, "file.title", 1, "Profile"),
                (ARCHIVES, "file.text", 1, "She leads Rhodes Island."),
                (ARCHIVES, "voice.title", 1, "Greeting"),
                (ARCHIVES, "voice.text", 1, "Hello, Doctor."),
                // A device has a name and no class.
                (OPERATORS, "name", 0, "Crate"),
            ]
        );
    }

    #[test]
    fn a_repeated_field_says_what_tells_its_texts_apart() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write(
            d,
            "character_table",
            r#"{"Characters":[{"key":"char_1","value":{"Name":"Amiya","TagList":["DPS","Support"],
            "Talents":[{"Candidates":[
              {"Name":"Absorption","Description":"Recovers 2 SP","RequiredPotentialRank":0,"UnlockCondition":{"Level":1,"Phase":"PHASE_2"}},
              {"Name":"Absorption","Description":"Recovers 3 SP","RequiredPotentialRank":5,"UnlockCondition":{"Level":1,"Phase":"PHASE_2"}}]}],
            "Skills":[{"SkillId":"sk1"}]}}]}"#,
        );
        let level = |text: &str| format!(r#"{{"Name":"Chant","Description":"{text}"}}"#);
        let levels: Vec<String> = (0..10)
            .map(|i| {
                level(if i < 7 {
                    "ASPD +{x}"
                } else {
                    "ASPD +{x}, ATK +{y}"
                })
            })
            .collect();
        write(
            d,
            "skill_table",
            &format!(
                r#"{{"Skills":[{{"key":"sk1","value":{{"Levels":[{}]}}}}]}}"#,
                levels.join(",")
            ),
        );
        write(
            d,
            "uniequip_table",
            r#"{"MissionList":[{"key":"m1","value":{"Desc":"Clear 1-1"}},{"key":"m2","value":{"Desc":"Clear 1-2"}}],
            "EquipDict":[{"key":"e1","value":{"CharId":"char_1","CharEquipOrder":1,"UniEquipId":"e1","UniEquipName":"Ring","MissionList":["m1","m2"]}}]}"#,
        );
        write(
            d,
            "battle_equip_table",
            r#"{"Equips":[{"key":"e1","value":{"Phases":[
              {"EquipLevel":1,"Parts":[{"X":{"Candidates":[{"AdditionalDescription":"Gains SP","RequiredPotentialRank":0,"UnlockCondition":{"Level":50,"Phase":"PHASE_2"}}]}}]},
              {"EquipLevel":2,"Parts":[{"X":{"Candidates":[{"AdditionalDescription":"Gains SP","RequiredPotentialRank":0,"UnlockCondition":{"Level":50,"Phase":"PHASE_2"}},
                {"UpgradeDescription":"Recovers 4 SP","RequiredPotentialRank":5,"UnlockCondition":{"Level":50,"Phase":"PHASE_2"}}]}}]}]}}]}"#,
        );
        write(
            d,
            "stage_table",
            r#"{"Stages":[{"key":"main_00-01","value":{"Code":"0-1","Name":"Collapse","Description":"Hold.","Difficulty":"NORMAL","DiffGroup":"NONE"}},
            {"key":"main_00-01#f#","value":{"Code":"0-1","Name":"Collapse","Description":"No DP.","Difficulty":"FOUR_STAR","DiffGroup":"NONE"}},
            {"key":"tough_10-01","value":{"Code":"10-2","Name":"Raid","Description":"Hard.","Difficulty":"NORMAL","DiffGroup":"TOUGH"}}]}"#,
        );
        write(
            d,
            "enemy_handbook_table",
            r#"{"EnemyData":[{"key":"enemy_1","value":{"Name":"Slug","EnemyIndex":"B1","Description":"Slow."}}]}"#,
        );
        let all = places(d);
        let note = |field: &str, text: &str| -> &str {
            &all.iter()
                .find(|p| p.field == field && p.text == text)
                .unwrap_or_else(|| panic!("{field}: {text}"))
                .note
        };
        assert_eq!(note("tag", "DPS"), "N1");
        assert_eq!(note("tag", "Support"), "N2");
        // The name is the same at both: one place, the notes of both.
        assert_eq!(note("talent.name", "Absorption"), "E2,E2+P6");
        assert_eq!(note("talent.description", "Recovers 2 SP"), "E2");
        assert_eq!(note("talent.description", "Recovers 3 SP"), "E2+P6");
        assert_eq!(
            note("skill.description", "ASPD +{x}"),
            "S1,S2,S3,S4,S5,S6,S7"
        );
        assert_eq!(note("skill.description", "ASPD +{x}, ATK +{y}"), "M1,M2,M3");
        assert_eq!(note("module.mission", "Clear 1-2"), "N2");
        assert_eq!(note("module.effect", "Gains SP"), "T1,T2");
        assert_eq!(note("module.effect", "Recovers 4 SP"), "T2+P6");
        assert_eq!(note("description", "Hold."), "");
        assert_eq!(note("description", "No DP."), "C");
        assert_eq!(note("description", "Hard."), "DT");
        assert_eq!(note("description", "Slow."), "#B1");
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
