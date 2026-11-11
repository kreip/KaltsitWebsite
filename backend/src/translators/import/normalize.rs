//! Text normalisation for matching an English story line to a dictionary key.
//!
//! Two shapes (translators/README.md, "XUnity dictionary and plugin format"):
//!
//! * [`plain`] is RichTextFix's `PlainText`: tags and Arknights macros
//!   removed, literal `\n` / `\r\n` turned into LF, spaces around LF removed,
//!   trimmed. The plugin's catalog is keyed by it.
//! * [`for_search`] is the review kit's search normalisation without its
//!   Cyrillic look-alike step, which only matters for Russian text: tags
//!   removed, every newline a space, `…` → `...`, every quote `"`, every dash
//!   `-`, NBSP a space, spaces collapsed, `ё` → `е`, lower case.

/// Remove `<...>` tags (TMP / uGUI and `<@...>`, `<$...>`, `</>`), keeping
/// `<br>` as a newline. A `<` with no closing `>` is kept as text.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        match tail.find('>') {
            Some(end) if !tail[1..end].contains('<') => {
                let tag = &tail[1..end];
                if tag.eq_ignore_ascii_case("br") || tag.eq_ignore_ascii_case("br/") {
                    out.push('\n');
                }
                rest = &tail[end + 1..];
            }
            _ => {
                out.push('<');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// RichTextFix's `PlainText`.
pub fn plain(s: &str) -> String {
    let s = strip_tags(s)
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\r\n", "\n");
    s.split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

/// The kit's search key.
pub fn for_search(s: &str) -> String {
    let s = strip_tags(s)
        .replace("\\r", " ")
        .replace("\\n", " ")
        .replace('…', "...");
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for c in s.chars() {
        let c = match c {
            '\n' | '\r' | '\t' | '\u{a0}' | '\u{2007}' | '\u{202f}' => ' ',
            '“' | '”' | '„' | '«' | '»' | '「' | '」' | '『' | '』' | '\'' | '‘' | '’' | '`' => {
                '"'
            }
            '–' | '—' | '―' | '‒' | '−' => '-',
            'ё' => 'е',
            'Ё' => 'Е',
            c => c,
        };
        if c == ' ' {
            if !out.is_empty() {
                space = true;
            }
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.extend(c.to_lowercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_strips_tags_and_trims_lines() {
        assert_eq!(
            plain("<i>Hello</i> \\n  <@ba.kw>world</>  "),
            "Hello\nworld"
        );
        assert_eq!(plain("a <b"), "a <b");
        assert_eq!(plain("a<br>b"), "a\nb");
    }

    #[test]
    fn search_key_folds_punctuation_and_case() {
        assert_eq!(for_search("“Wait—”\n<i>Doctor</i>…"), "\"wait-\" doctor...");
        assert_eq!(for_search("  It's  "), "it\"s");
        assert_eq!(for_search("Ёж"), "еж");
    }
}
