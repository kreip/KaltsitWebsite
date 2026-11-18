//! The rules of line editing, each in one place: a line's status, a proposal's state, which lines can be edited
//! and when an edit needs a confirmation.

use crate::translators::import::normalize::plain;

/// More places than this: a proposal for every place is confirmed separately,
/// with the list (a proposal for one place never is).
pub const CONFIRM_PLACES: i32 = 1;

/// The longest quoted fragment of a comment, in characters.
pub const MAX_QUOTE: usize = 500;

/// The longest proposal, in characters: an operator's file runs to 7,000.
pub const MAX_TEXT: usize = 20_000;

/// The longest comment, in characters.
pub const MAX_COMMENT: usize = 4_000;

/// A line's status, by its edit key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStatus {
    /// Nobody has looked at it yet (the default).
    Unchecked,
    /// At least one live proposal.
    Proposed,
    /// The lead approved a variant (the game's Russian included).
    Approved,
    /// An approval or proposal made at this place is for an English text the
    /// story no longer has here.
    Outdated,
}

impl LineStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unchecked => "unchecked",
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Outdated => "outdated",
        }
    }
}

/// Approved wins (the line's current text is decided); then an outdated
/// decision left at this place; then live proposals.
pub fn line_status(approved: bool, outdated_here: bool, proposals: usize) -> LineStatus {
    if approved {
        LineStatus::Approved
    } else if outdated_here {
        LineStatus::Outdated
    } else if proposals > 0 {
        LineStatus::Proposed
    } else {
        LineStatus::Unchecked
    }
}

/// One proposal's state for "My edits".
pub fn proposal_state(outdated: bool, approved: Option<Option<i64>>, id: i64) -> &'static str {
    if outdated {
        return "outdated";
    }
    match approved {
        Some(Some(p)) if p == id => "approved",
        // Another variant (or the game's Russian) was approved.
        Some(_) => "otherApproved",
        None => "proposed",
    }
}

/// A line with no letter (dots, dashes, an ellipsis, a number; tags and
/// substitutions `{…}` aside) is not edited: there is nothing to translate.
/// It counts as checked by the system (`translators.lettered` in SQL, the
/// same rule).
pub fn editable(edit_key: &str) -> bool {
    let text = plain(edit_key);
    let mut depth = 0u32;
    text.chars().any(|c| match c {
        '{' => {
            depth += 1;
            false
        }
        '}' => {
            depth = depth.saturating_sub(1);
            false
        }
        c => depth == 0 && c.is_alphabetic(),
    })
}

/// The same text, typography aside — spaces, «…» / «...», the kind of quotes
/// and dashes (the page's `sameText`): a variant equal to the game's text.
pub fn same_text(a: &str, b: &str) -> bool {
    fn squash(x: &str) -> String {
        let x = x.replace("\\n", "\n").replace('…', "...");
        let mut out = String::with_capacity(x.len());
        let mut space = false;
        for c in x.trim().chars() {
            let c = match c {
                '«' | '»' | '“' | '”' | '„' => '"',
                '—' | '–' | '‑' => '-',
                c => c,
            };
            if c.is_whitespace() {
                space = true;
                continue;
            }
            if space && !out.is_empty() {
                out.push(' ');
            }
            space = false;
            out.push(c);
        }
        out
    }
    squash(a) == squash(b)
}

/// A proposal's text as stored: line breaks as LF, trimmed. `None` when
/// empty or too long.
pub fn clean_text(text: &str) -> Option<String> {
    clean_up_to(text, MAX_TEXT)
}

/// A comment's text as stored, as a proposal's. `None` when empty or too long.
pub fn clean_comment(text: &str) -> Option<String> {
    clean_up_to(text, MAX_COMMENT)
}

fn clean_up_to(text: &str, max: usize) -> Option<String> {
    let t = text.replace("\r\n", "\n").replace('\r', "\n");
    let t = t.trim();
    (!t.is_empty() && t.chars().count() <= max).then(|| t.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_precedence() {
        assert_eq!(line_status(false, false, 0), LineStatus::Unchecked);
        assert_eq!(line_status(false, false, 2), LineStatus::Proposed);
        assert_eq!(line_status(false, true, 2), LineStatus::Outdated);
        assert_eq!(line_status(true, true, 2), LineStatus::Approved);
    }

    #[test]
    fn proposal_states() {
        assert_eq!(proposal_state(false, None, 1), "proposed");
        assert_eq!(proposal_state(false, Some(Some(1)), 1), "approved");
        assert_eq!(proposal_state(false, Some(None), 1), "otherApproved");
        assert_eq!(proposal_state(false, Some(Some(2)), 1), "otherApproved");
        assert_eq!(proposal_state(true, Some(Some(1)), 1), "outdated");
    }

    #[test]
    fn punctuation_only_lines_are_not_edited() {
        assert!(!editable("……"));
        assert!(!editable("—!?"));
        assert!(!editable("<i>...</i>"));
        assert!(editable("Hm."));
        // A number, a substitution: nothing to translate either.
        assert!(!editable("3"));
        assert!(!editable("{0}%"));
        assert!(!editable("...?"));
        assert!(editable("{0} left"));
    }

    #[test]
    fn typography_aside_texts_are_the_same() {
        assert!(same_text("Доктор...?", "Доктор…?"));
        assert!(same_text("«Да» — сказал он.", "\"Да\" - сказал  он."));
        assert!(!same_text("Доктор?", "Доктор…?"));
    }

    #[test]
    fn texts_are_cleaned() {
        assert_eq!(clean_text("  Да.\r\nНет. ").as_deref(), Some("Да.\nНет."));
        assert_eq!(clean_text("   "), None);
        assert_eq!(clean_text(&"а".repeat(MAX_TEXT + 1)), None);
        assert!(clean_text(&"а".repeat(MAX_COMMENT + 1)).is_some());
        assert_eq!(clean_comment(&"а".repeat(MAX_COMMENT + 1)), None);
    }
}
