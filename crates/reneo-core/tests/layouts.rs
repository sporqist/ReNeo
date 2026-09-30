//! Keysyms and layouts against the shipped data files, ported from the D version's source/tests.d

mod common;

use common::{keysyms, repo_file};
use reneo_core::keys::{Modifier, Scancode};
use reneo_core::keysym::KEYSYM_VOID;
use reneo_core::layout::{parse_layouts, KeyAction};

// ─── Keysyms ───────────────────────────────────────────────────────────────────

#[test]
fn keysyms_from_keysymdef() {
    let keysyms = keysyms();
    assert_eq!(keysyms.parse_keysym("a"), 0x61);
    assert_eq!(keysyms.parse_keysym("Multi_key"), 0xFF20);
    assert_eq!(keysyms.parse_keysym("U20AC"), 0x20AC);
    assert_eq!(keysyms.parse_keysym("U1F574"), 0x0101_F574);
    assert_eq!(keysyms.parse_keysym("does_not_exist"), KEYSYM_VOID);
}

// ─── Layouts ───────────────────────────────────────────────────────────────────

#[test]
fn shipped_layouts_load() {
    let keysyms = keysyms();
    let layouts = parse_layouts(&std::fs::read_to_string(repo_file("layouts.json")).unwrap(), &keysyms).unwrap();
    let names: Vec<_> = layouts.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Neo", "Bone", "NeoQwertz", "AdNW", "Mine", "KOY", "VOU", "3l"]);

    for layout in &layouts {
        for entry in layout.map.values() {
            assert_eq!(entry.layers.len(), layout.layers.len(), "{}", layout.name);
        }
    }

    let neo = &layouts[0];
    assert_eq!(neo.dll_name.as_deref(), Some("kbdneo2.dll"));
    assert_eq!(neo.modifiers[&Scancode::new(0x38, true)], Modifier::RMOD4);
    assert!(neo.is_capslockable(Scancode::new(0x10, false)));
    // Neo layer 4 on the i key is the left arrow
    assert_eq!(neo.key(Scancode::new(0x1F, false), 4).unwrap().action, KeyAction::Vk { vk: 0x25, mods: Default::default() });
    assert_eq!(neo.key(Scancode::new(0x20, false), 3).unwrap().action, KeyAction::Char('{'));
}

#[test]
fn every_shipped_keysym_is_known() {
    let keysyms = keysyms();
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(repo_file("layouts.json")).unwrap()).unwrap();
    let mut unknown = Vec::new();
    for layout in json["layouts"].as_array().unwrap() {
        for (scancode, keys) in layout["map"].as_object().unwrap() {
            for key in keys.as_array().unwrap() {
                if let Some(name) = key.get("keysym").and_then(|k| k.as_str()) {
                    if name != "VoidSymbol" && keysyms.parse_keysym(name) == KEYSYM_VOID {
                        unknown.push(format!("{}/{scancode}: {name}", layout["name"]));
                    }
                }
            }
        }
    }
    assert!(unknown.is_empty(), "{unknown:?}");
}

#[test]
fn invalid_layouts_give_helpful_errors() {
    let keysyms = keysyms();
    let error = parse_layouts(r#"{"layouts": [{
        "name": "Broken",
        "modifiers": {"2A": "LShift"},
        "layers": [{"Shift": false}, {"Shift": true}],
        "capslockableKeys": ["10", "11"],
        "map": {"10": [{"keysym": "x", "char": "x"}, {"keysym": "X", "char": "X"}]}
    }]}"#, &keysyms).unwrap_err();
    assert!(error.contains("Broken") && error.contains("11"), "{error}");

    let error = parse_layouts(r#"{"layouts": [{
        "name": "Short",
        "modifiers": {},
        "layers": [{"Shift": false}, {"Shift": true}],
        "capslockableKeys": [],
        "map": {"10": [{"keysym": "x", "char": "x"}]}
    }]}"#, &keysyms).unwrap_err();
    assert!(error.contains("Short"), "{error}");

    let layouts = parse_layouts(r#"{"layouts": [{
        "name": "Emoji",
        "modifiers": {},
        "layers": [{}],
        "capslockableKeys": [],
        "map": {"10": [{"keysym": "U1F574", "char": "🕴"}]}
    }]}"#, &keysyms).unwrap();
    assert_eq!(layouts[0].key(Scancode::new(0x10, false), 1).unwrap().action, KeyAction::Char('🕴'));
}
