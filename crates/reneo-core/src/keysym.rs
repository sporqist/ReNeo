//! X11 keysyms: the names compose files use for keys ("Multi_key", "dead_acute", "U20AC").
//!
//! The table comes from `keysymdef.h` of the X.Org project, which ships with ReNeo.

use std::collections::HashMap;

/// Keysym of keys without a meaning ("VoidSymbol")
pub const KEYSYM_VOID: u32 = 0x00FF_FFFF;
/// Keysyms for Unicode characters without a legacy keysym are the codepoint plus this offset
pub const KEYSYM_CODEPOINT_OFFSET: u32 = 0x0100_0000;

#[derive(Debug, Default)]
pub struct Keysyms {
    by_name: HashMap<String, u32>,
    by_codepoint: HashMap<u32, u32>,
    codepoint_by_keysym: HashMap<u32, u32>,
}

impl Keysyms {
    /// Parses the `#define XK_<name> 0x<value>` lines of keysymdef.h. A comment of the form
    /// `/* U+20AC EURO SIGN */` or `/*(U+20AC …)*/` links the keysym to a Unicode codepoint.
    pub fn parse(text: &str) -> Keysyms {
        let mut keysyms = Keysyms::default();
        for line in text.lines() {
            let mut tokens = line.split_whitespace();
            if tokens.next() != Some("#define") {
                continue;
            }
            let Some(name) = tokens.next().and_then(|t| t.strip_prefix("XK_")) else { continue };
            let Some(value) = tokens
                .next()
                .and_then(|t| t.strip_prefix("0x"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            else {
                continue;
            };

            keysyms.by_name.insert(name.to_string(), value);
            if let Some(codepoint) = comment_codepoint(line) {
                keysyms.by_codepoint.insert(codepoint, value);
                keysyms.codepoint_by_keysym.insert(value, codepoint);
            }
        }
        keysyms
    }

    /// Looks up a keysym by name, or by codepoint for names of the form "U00A0" to "U10FFFF".
    /// Unknown names give [`KEYSYM_VOID`].
    pub fn parse_keysym(&self, name: &str) -> u32 {
        if let Some(&keysym) = self.by_name.get(name) {
            return keysym;
        }
        if let Some(codepoint) = name
            .strip_prefix('U')
            .filter(|hex| !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()))
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        {
            // Some Unicode values between 0x0100 and 0x30FF have legacy keysyms
            if codepoint <= 0x30FF {
                if let Some(&keysym) = self.by_codepoint.get(&codepoint) {
                    return keysym;
                }
            }
            return codepoint + KEYSYM_CODEPOINT_OFFSET;
        }
        KEYSYM_VOID
    }

    /// The character a keysym stands for, if any
    pub fn to_char(&self, keysym: u32) -> Option<char> {
        if let Some(&codepoint) = self.codepoint_by_keysym.get(&keysym) {
            return char::from_u32(codepoint);
        }
        if keysym > KEYSYM_CODEPOINT_OFFSET {
            return char::from_u32(keysym - KEYSYM_CODEPOINT_OFFSET);
        }
        None
    }
}

/// Extracts the codepoint from comments like `/* U+20AC EURO SIGN */` and `/*(U+20AC …)*/`.
/// The "U+" must directly follow the comment start and one space or parenthesis, like in keysymdef.h.
fn comment_codepoint(line: &str) -> Option<u32> {
    let comment = &line[line.find("/*")? + 2..];
    let rest = comment.strip_prefix(' ').or_else(|| comment.strip_prefix('('))?;
    let hex = rest.strip_prefix("U+")?;
    let digits = hex.chars().take_while(|c| c.is_ascii_hexdigit()).count();
    if !(4..=6).contains(&digits) || !hex[digits..].starts_with(' ') {
        return None;
    }
    u32::from_str_radix(&hex[..digits], 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
#define XK_a                             0x0061  /* U+0061 LATIN SMALL LETTER A */
#define XK_Multi_key                     0xff20  /* Multi-key character compose */
#define XK_EuroSign                      0x20ac  /* U+20AC EURO SIGN */
#define XK_Babovedot                  0x1001e02  /* U+1E02 LATIN CAPITAL LETTER B WITH DOT ABOVE */
#define XK_braille_dot_1                 0xfff1
#define XK_ch                          0xfea0
#define XK_Greek_ALPHAaccent             0x07a1  /*(U+0386 GREEK CAPITAL LETTER ALPHA WITH TONOS)*/
";

    #[test]
    fn parses_names_and_codepoints() {
        let keysyms = Keysyms::parse(SAMPLE);
        assert_eq!(keysyms.parse_keysym("a"), 0x61);
        assert_eq!(keysyms.parse_keysym("Multi_key"), 0xFF20);
        assert_eq!(keysyms.parse_keysym("braille_dot_1"), 0xFFF1);
        assert_eq!(keysyms.to_char(0x07A1), Some('Ά'));
        assert_eq!(keysyms.to_char(0xFF20), None);
    }

    #[test]
    fn parses_unicode_names() {
        let keysyms = Keysyms::parse(SAMPLE);
        // Legacy keysym for a codepoint below 0x3100
        assert_eq!(keysyms.parse_keysym("U20AC"), 0x20AC);
        assert_eq!(keysyms.parse_keysym("U1F574"), 0x0101_F574);
        assert_eq!(keysyms.to_char(0x0101_F574), Some('🕴'));
        assert_eq!(keysyms.parse_keysym("does_not_exist"), KEYSYM_VOID);
        assert_eq!(keysyms.parse_keysym("U"), KEYSYM_VOID);
    }
}
