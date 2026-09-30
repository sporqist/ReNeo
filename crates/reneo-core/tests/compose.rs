//! Compose against the shipped modules, ported from the D version's source/tests.d

mod common;

use common::{keysyms, repo_file};
use reneo_core::compose::{parse_line, Compose, ComposeResult};
use reneo_core::keysym::Keysyms;
use reneo_core::layout::NeoKey;

fn key(keysyms: &Keysyms, name: &str) -> NeoKey {
    NeoKey { keysym: keysyms.parse_keysym(name), ..NeoKey::void() }
}

fn compose_sequence(compose: &mut Compose, keysyms: &Keysyms, names: &[&str]) -> (ComposeResult, String) {
    let mut result = ComposeResult::Pass;
    for name in names {
        result = compose.compose(&key(keysyms, name), keysyms);
    }
    (result, compose.output().to_string())
}

// ─── Compose ───────────────────────────────────────────────────────────────────

#[test]
fn parses_compose_lines() {
    let keysyms = keysyms();
    let entry = parse_line(r#"<Multi_key> <a> <e>  : "æ"  ae # LATIN SMALL LETTER AE"#, &keysyms).unwrap().unwrap();
    assert_eq!(entry.keysyms, [0xFF20, 0x61, 0x65]);
    assert_eq!(entry.result, "æ");

    let escaped = parse_line(r#"<Multi_key> <backslash> <n> : "\n\"\\""#, &keysyms).unwrap().unwrap();
    assert_eq!(escaped.result, "\n\"\\");

    assert_eq!(parse_line("# just a comment", &keysyms), Ok(None));
    assert_eq!(parse_line("", &keysyms), Ok(None));

    // Malformed lines are errors, never a hang or a panic
    assert!(parse_line("<Multi_key", &keysyms).is_err());
    assert!(parse_line(r#"<Multi_key> <a> "x""#, &keysyms).is_err());
    assert!(parse_line(r#"<Multi_key> <a> : "unterminated"#, &keysyms).is_err());
    assert!(parse_line(r#"<Multi_key> <a> : "ends with backslash\"#, &keysyms).is_err());
}

#[test]
fn composes_with_shipped_modules() {
    let keysyms = keysyms();
    let mut compose = Compose::load(&repo_file("compose"), &keysyms).unwrap();
    assert!(compose.added_entries > 1000);

    assert_eq!(compose_sequence(&mut compose, &keysyms, &["Multi_key", "o", "c"]), (ComposeResult::Finish, "©".into()));

    // Unknown sequences are aborted and the typed characters are output
    assert_eq!(compose_sequence(&mut compose, &keysyms, &["Multi_key", "o", "Q"]), (ComposeResult::Abort, "oQ".into()));

    // Escape aborts without output
    assert_eq!(compose_sequence(&mut compose, &keysyms, &["Multi_key", "o", "Escape"]), (ComposeResult::Abort, "".into()));

    // Keys outside of compose sequences pass
    assert_eq!(compose.compose(&key(&keysyms, "a"), &keysyms), ComposeResult::Pass);
}

#[test]
fn special_compose_modes() {
    let keysyms = keysyms();
    let mut compose = Compose::load(&repo_file("compose"), &keysyms).unwrap();

    let unicode = ["Multi_key", "u", "u", "1", "f", "5", "7", "4", "space"];
    assert_eq!(compose_sequence(&mut compose, &keysyms, &unicode), (ComposeResult::Finish, "🕴".into()));

    let roman = ["Multi_key", "r", "n", "1", "9", "7", "0", "space"];
    assert_eq!(compose_sequence(&mut compose, &keysyms, &roman), (ComposeResult::Finish, "ⅿⅽⅿⅼⅹⅹ".into()));

    assert_eq!(compose_sequence(&mut compose, &keysyms, &["Multi_key", "R", "N", "4", "space"]), (ComposeResult::Finish, "ⅠⅤ".into()));

    // 0 is not a roman numeral, and surrogates are no characters
    assert_eq!(compose_sequence(&mut compose, &keysyms, &["Multi_key", "r", "n", "0", "space"]).0, ComposeResult::Abort);
    assert_eq!(compose_sequence(&mut compose, &keysyms, &["Multi_key", "u", "u", "d", "8", "0", "0", "space"]).0, ComposeResult::Abort);
}

#[test]
fn compose_reset_and_reload() {
    let keysyms = keysyms();
    let mut compose = Compose::load(&repo_file("compose"), &keysyms).unwrap();

    // A partial sequence must not survive a reset (used after lost input)
    compose.compose(&key(&keysyms, "Multi_key"), &keysyms);
    compose.reset();
    assert_eq!(compose.compose(&key(&keysyms, "a"), &keysyms), ComposeResult::Pass);

    // Loading twice gives the same table
    let again = Compose::load(&repo_file("compose"), &keysyms).unwrap();
    assert_eq!(again.added_entries, compose.added_entries);
}
