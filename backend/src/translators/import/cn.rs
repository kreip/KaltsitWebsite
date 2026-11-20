//! The Chinese original from the CN client's own data, read from a folder the
//! site's downloader filled (`downloader --server cn … --profile gamedata`,
//! then `unpacker extract --gamedata`): the story scripts at the same paths as
//! the EN ones, the excel tables of the same shape. The site never serves the
//! CN server (it would hold its whole data in memory): only the import reads
//! the folder (`TRANSLATORS_CN_DIR`).
//!
//! A story's CN lines are paired with its EN lines in order. Most scripts show
//! the same texts in the same order ("order"); where the two files differ
//! (a line split, a scene the EN client lacks) an alignment pairs lines of the
//! same kind, preferring the same line number ("line") and a speaker whose CN
//! name the corpus pairs with the EN one ("aligned"). A line left unpaired has
//! no CN rather than a wrong one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::lines::{Line, displayed};

/// The CN client's output folder (`…/output/cn`), when it holds game data.
pub fn dir_of(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|p| p.join("gamedata").join("story").is_dir())
}

/// The CN excel tables (`gamedata/excel`).
pub fn excel(dir: &Path) -> PathBuf {
    dir.join("gamedata").join("excel")
}

/// A story's displayed CN lines, as [`displayed`] picks them, when the CN client has the script.
pub fn script_lines(dir: &Path, story_txt: &str) -> Option<Vec<Line>> {
    let script = crate::core::story::load_script(dir, story_txt).ok()?;
    let lines = displayed(&crate::core::story::parser::parse(&script));
    (!lines.is_empty()).then_some(lines)
}

/// How an EN line found its CN line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// The two scripts show the same kinds in the same order.
    Order,
    /// Paired by the alignment at the same line number.
    Line,
    /// Paired by the alignment elsewhere: the same kind between paired lines.
    Aligned,
}

impl How {
    pub fn word(self) -> &'static str {
        match self {
            How::Order => "order",
            How::Line => "line",
            How::Aligned => "aligned",
        }
    }
}

/// Which CN speaker name the corpus pairs with each EN one, counted over the
/// stories paired in order (`Amiya` -> `阿米娅`).
#[derive(Debug, Default)]
pub struct Speakers {
    votes: HashMap<String, HashMap<String, u32>>,
}

impl Speakers {
    pub fn vote(&mut self, en: &[Line], cn: &[Line], pairs: &[Option<(usize, How)>]) {
        for (i, p) in pairs.iter().enumerate() {
            let Some((j, _)) = p else { continue };
            let (e, c) = (&en[i].speaker_en, &cn[*j].speaker_en);
            if !e.is_empty() && !c.is_empty() {
                *self
                    .votes
                    .entry(e.clone())
                    .or_default()
                    .entry(c.clone())
                    .or_default() += 1;
            }
        }
    }

    /// The CN name the EN speaker usually has (at least two lines say so).
    pub fn best(&self, en: &str) -> Option<String> {
        let (name, n) = self.votes.get(en)?.iter().max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))?;
        (*n >= 2).then(|| name.clone())
    }

    /// Whether `cn` is the CN name the EN speaker `en` usually has: `None` when not known.
    fn agrees(&self, en: &str, cn: &str) -> Option<bool> {
        let by = self.votes.get(en)?;
        let (best, n) = by.iter().max_by_key(|(_, n)| **n)?;
        (*n >= 2).then(|| best == cn || by.get(cn).is_some_and(|m| *m * 4 >= *n))
    }
}

fn same_kind(a: &Line, b: &Line) -> bool {
    a.kind == b.kind && a.part == b.part
}

/// Pairs in order when both scripts show the same kinds in the same order.
pub fn in_order(en: &[Line], cn: &[Line]) -> Option<Vec<Option<(usize, How)>>> {
    (en.len() == cn.len() && en.iter().zip(cn).all(|(a, b)| same_kind(a, b)))
        .then(|| (0..en.len()).map(|i| Some((i, How::Order))).collect())
}

/// For each EN line, its CN line and how it was found.
pub fn align(en: &[Line], cn: &[Line], speakers: &Speakers) -> Vec<Option<(usize, How)>> {
    if let Some(p) = in_order(en, cn) {
        return p;
    }
    let (n, m) = (en.len(), cn.len());
    // The score of pairing EN i with CN j, `None` when the kinds differ.
    let score = |i: usize, j: usize| -> Option<i32> {
        let (a, b) = (&en[i], &cn[j]);
        if !same_kind(a, b) {
            return None;
        }
        let mut s = 2;
        if a.line == b.line {
            s += 3;
        }
        match (a.speaker_en.is_empty(), b.speaker_en.is_empty()) {
            (true, true) => s += 1,
            (false, false) => match speakers.agrees(&a.speaker_en, &b.speaker_en) {
                Some(true) => s += 3,
                Some(false) => s -= 3,
                None => {}
            },
            _ => s -= 2,
        }
        (s > 0).then_some(s)
    };
    // Needleman-Wunsch without gap costs: the best sum of pair scores.
    let w = m + 1;
    let mut best = vec![0i32; (n + 1) * w];
    // 0 = pair, 1 = skip EN, 2 = skip CN.
    let mut step = vec![0u8; (n + 1) * w];
    for i in 0..=n {
        for j in 0..=m {
            if i == 0 && j == 0 {
                continue;
            }
            let mut b = i32::MIN;
            let mut s = 0u8;
            if i > 0 && j > 0
                && let Some(p) = score(i - 1, j - 1)
            {
                b = best[(i - 1) * w + j - 1] + p;
                s = 0;
            }
            if i > 0 && best[(i - 1) * w + j] > b {
                b = best[(i - 1) * w + j];
                s = 1;
            }
            if j > 0 && best[i * w + j - 1] > b {
                b = best[i * w + j - 1];
                s = 2;
            }
            best[i * w + j] = b;
            step[i * w + j] = s;
        }
    }
    let mut out = vec![None; n];
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        match step[i * w + j] {
            0 if i > 0 && j > 0 => {
                let how = if en[i - 1].line == cn[j - 1].line {
                    How::Line
                } else {
                    How::Aligned
                };
                out[i - 1] = Some((j - 1, how));
                i -= 1;
                j -= 1;
            }
            1 if i > 0 => i -= 1,
            _ if j > 0 => j -= 1,
            _ => i -= 1,
        }
    }
    out
}

/// Put the CN text and speaker on the paired EN lines.
pub fn apply(en: &mut [Line], cn: &[Line], pairs: &[Option<(usize, How)>]) {
    for (l, p) in en.iter_mut().zip(pairs) {
        if let Some((j, _)) = p {
            let c = &cn[*j];
            l.cn = Some(c.en.clone());
            l.speaker_cn = (!c.speaker_en.is_empty()).then(|| c.speaker_en.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(line: u32, kind: &'static str, speaker: &str, text: &str) -> Line {
        Line {
            line,
            kind,
            speaker_en: speaker.into(),
            en: text.into(),
            ..Line::default()
        }
    }

    #[test]
    fn the_same_shape_pairs_in_order() {
        let en = vec![l(3, "dialog", "Amiya", "Doctor?"), l(5, "narration", "", "Rain.")];
        let cn = vec![l(4, "dialog", "阿米娅", "博士？"), l(6, "narration", "", "下雨了。")];
        let p = align(&en, &cn, &Speakers::default());
        assert_eq!(p, vec![Some((0, How::Order)), Some((1, How::Order))]);
    }

    #[test]
    fn a_line_only_one_side_has_is_skipped() {
        let mut sp = Speakers::default();
        let en0 = vec![l(1, "dialog", "Amiya", "A"), l(2, "dialog", "Kal'tsit", "B")];
        let cn0 = vec![l(1, "dialog", "阿米娅", "甲"), l(2, "dialog", "凯尔希", "乙")];
        for _ in 0..2 {
            let p = in_order(&en0, &cn0).unwrap();
            sp.vote(&en0, &cn0, &p);
        }
        // CN splits Amiya's line in two and has an extra narration.
        let en = vec![
            l(10, "dialog", "Amiya", "Hello. How are you?"),
            l(11, "dialog", "Kal'tsit", "Fine."),
            l(12, "narration", "", "They leave."),
        ];
        let cn = vec![
            l(10, "dialog", "阿米娅", "你好。"),
            l(11, "dialog", "阿米娅", "你好吗？"),
            l(12, "dialog", "凯尔希", "还行。"),
            l(13, "narration", "", "风很大。"),
            l(14, "narration", "", "他们离开了。"),
        ];
        let p = align(&en, &cn, &sp);
        assert_eq!(p[0], Some((0, How::Line)));
        assert_eq!(p[1], Some((2, How::Aligned)));
        assert!(p[2].is_some());
        let mut en = en;
        apply(&mut en, &cn, &p);
        assert_eq!(en[1].cn.as_deref(), Some("还行。"));
        assert_eq!(en[1].speaker_cn.as_deref(), Some("凯尔希"));
    }
}

/// The pairing over the real clients' files: `TRANSLATORS_EN_ASSETS` (`…/output/en`) and
/// `TRANSLATORS_CN_DIR` (`…/output/cn`). Prints the counts; run with `--ignored --nocapture`.
#[cfg(test)]
mod real {
    use super::*;

    #[test]
    #[ignore = "needs the EN and CN clients' data"]
    fn real_files() {
        let en_dir = PathBuf::from(std::env::var("TRANSLATORS_EN_ASSETS").unwrap());
        let cn_dir = PathBuf::from(std::env::var("TRANSLATORS_CN_DIR").unwrap());
        let table: serde_json::Value = serde_json::from_slice(
            &std::fs::read(en_dir.join("gamedata/excel/story_review_table.json")).unwrap(),
        )
        .unwrap();
        let mut txts: Vec<String> = Vec::new();
        for g in table["Story_reviews"].as_array().unwrap() {
            for d in g["value"]["InfoUnlockDatas"].as_array().into_iter().flatten() {
                if let Some(t) = d["StoryTxt"].as_str() {
                    txts.push(t.to_owned());
                }
            }
        }
        txts.sort();
        txts.dedup();
        let mut stories = Vec::new();
        for t in &txts {
            let Ok(script) = crate::core::story::load_script(&en_dir, t) else { continue };
            let en = displayed(&crate::core::story::parser::parse(&script));
            stories.push((t.clone(), en, script_lines(&cn_dir, t)));
        }
        let mut sp = Speakers::default();
        for (_, en, cn) in &stories {
            if let Some(c) = cn
                && let Some(p) = in_order(en, c)
            {
                sp.vote(en, c, &p);
            }
        }
        let (mut lines, mut missing, mut unpaired) = (0, 0, 0);
        let mut by: HashMap<&str, usize> = HashMap::new();
        let mut worst: Vec<(usize, String, usize)> = Vec::new();
        for (t, en, cn) in &stories {
            lines += en.len();
            let Some(c) = cn else {
                missing += en.len();
                continue;
            };
            let p = align(en, c, &sp);
            let mut un = 0;
            for x in &p {
                match x {
                    Some((_, h)) => *by.entry(h.word()).or_default() += 1,
                    None => un += 1,
                }
            }
            unpaired += un;
            if un > 0 {
                worst.push((un, t.clone(), en.len()));
            }
        }
        worst.sort_by(|a, b| b.0.cmp(&a.0));
        println!("stories {} lines {lines} no CN script {missing} unpaired {unpaired} by {by:?}", stories.len());
        for w in worst.iter().take(25) {
            println!("  {} of {} unpaired: {}", w.0, w.2, w.1);
        }
    }
}
