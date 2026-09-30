//! Key handling, ported from the D version's source/tests.d and extended. Uses the shipped Neo layout and a fake
//! platform, so the tests run without Windows.

mod common;

use std::collections::HashMap;

use common::{keysyms, repo_file};
use reneo_core::compose::Compose;
use reneo_core::engine::{CharKey, Engine, EngineConfig, Input, KeyEvent, Notice, Output, Platform};
use reneo_core::keys::{vk, ForcedModifiers, Modifier, Scancode};
use reneo_core::layout::parse_layouts;

// ─── Harness ───────────────────────────────────────────────────────────────────

#[derive(Default)]
struct FakePlatform {
    capslock: bool,
    kana: bool,
    /// Native key combinations for standalone mode, everything else is sent as Unicode
    chars: HashMap<char, CharKey>,
}

impl Platform for FakePlatform {
    fn capslock(&self) -> bool {
        self.capslock
    }
    fn kana(&self) -> bool {
        self.kana
    }
    fn vk_to_scan(&self, _vk: u16) -> u16 {
        0
    }
    fn char_key(&mut self, c: char, _capslock: bool) -> Option<CharKey> {
        self.chars.get(&c).copied()
    }
}

struct Test {
    engine: Engine,
    platform: FakePlatform,
    out: Output,
}

// Scancodes of the Neo layout
const LMOD3: u16 = 0x3A; // Capslock position
const LMOD4: u16 = 0x56; // ISO key left of Y/Z
const RMOD4: u16 = 0x38; // AltGr (extended)
const LSHIFT: u16 = 0x2A;
const RSHIFT: u16 = 0x36; // extended
const TAB: u16 = 0x0F;
const SPACE: u16 = 0x39;
const KEY_A: u16 = 0x20; // Neo: a, {, Down
const KEY_E: u16 = 0x21; // Neo: e, }, Right
const KEY_I: u16 = 0x1F; // Neo: i, /, Left
const KEY_SZ: u16 = 0x1A; // Neo: ß, ẞ

impl Test {
    fn new(standalone: bool) -> Test {
        Test::with_config(standalone, EngineConfig::default())
    }

    fn with_config(standalone: bool, config: EngineConfig) -> Test {
        let keysyms = keysyms();
        let layouts = parse_layouts(&std::fs::read_to_string(repo_file("layouts.json")).unwrap(), &keysyms).unwrap();
        let compose = Compose::load(&repo_file("compose"), &keysyms).unwrap();
        let mut engine = Engine::new(layouts, keysyms, compose, config);
        engine.set_active_layout(Some(0));
        engine.set_standalone(standalone);
        Test { engine, platform: FakePlatform::default(), out: Output::default() }
    }

    /// Feeds one physical key event, returns whether it was swallowed. The output stays in `self.out`.
    fn event(&mut self, vk_code: u16, scan: u16, extended: bool, down: bool) -> bool {
        self.out.clear();
        let event = KeyEvent { vk: vk_code, scan: Scancode::new(scan, extended), down, injected: false };
        self.engine.handle(&event, &mut self.platform, &mut self.out)
    }

    fn key(&mut self, scan: u16, down: bool) -> bool {
        self.event(0, scan, false, down)
    }

    fn ext_key(&mut self, scan: u16, down: bool) -> bool {
        self.event(0, scan, true, down)
    }

    fn tap(&mut self, scan: u16) -> Vec<Ev> {
        self.key(scan, true);
        let mut events = self.events();
        self.key(scan, false);
        events.extend(self.events());
        events
    }

    /// The output of the last event, simplified
    fn events(&self) -> Vec<Ev> {
        self.out.inputs.iter().map(|input| match *input {
            Input::Key { vk, down, .. } => Ev::Vk(vk, down),
            Input::Unicode { unit, down } => Ev::Uni(char::from_u32(u32::from(unit)).unwrap_or('�'), down),
            Input::MouseLeft { down } => Ev::Mouse(down),
        }).collect()
    }

    /// Scancode of the key producing a keysym on layer 1 of the active layout
    fn scancode_for(&self, keysym_name: &str) -> u16 {
        let keysym = keysyms().parse_keysym(keysym_name);
        let layout = self.engine.active_layout().unwrap();
        layout.map.iter().find(|(_, entry)| entry.layers[0].keysym == keysym).map(|(scan, _)| scan.code).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ev {
    Vk(u16, bool),
    Uni(char, bool),
    Mouse(bool),
}

use Ev::{Uni, Vk};
const D: bool = true;
const U: bool = false;

// ─── Extension mode ────────────────────────────────────────────────────────────

#[test]
fn layer4_navigation_in_extension_mode() {
    let mut t = Test::new(false);
    assert!(t.key(LMOD4, D), "Mod4 is filtered");
    assert_eq!(t.events(), []);

    assert!(t.key(KEY_I, D));
    assert_eq!(t.events(), [Vk(vk::LEFT, D)]);
    assert!(t.key(KEY_I, U));
    assert_eq!(t.events(), [Vk(vk::LEFT, U)]);
    assert!(t.key(LMOD4, U));

    // On layer 1, letters are left to the native layout
    assert!(!t.key(KEY_I, D));
    assert!(!t.key(KEY_I, U));
    assert_eq!(t.events(), []);
}

#[test]
fn layer3_characters_in_extension_mode_are_unicode() {
    let mut t = Test::new(false);
    t.key(LMOD3, D);
    assert!(t.key(KEY_A, D));
    assert_eq!(t.events(), [Uni('{', D)]);
    assert!(t.key(KEY_A, U));
    assert_eq!(t.events(), [Uni('{', U)]);
}

#[test]
fn compose_with_m3_tab() {
    let mut t = Test::new(false);
    t.key(LMOD3, D);
    assert!(t.key(TAB, D));
    t.key(TAB, U);
    t.key(LMOD3, U);

    let (o, c) = (t.scancode_for("o"), t.scancode_for("c"));
    assert!(t.key(o, D));
    assert!(t.key(o, U));
    assert_eq!(t.events(), []);
    assert!(t.key(c, D));
    assert_eq!(t.events(), [Uni('©', D), Uni('©', U)]);
    assert!(t.key(c, U));
}

#[test]
fn mod4_lock() {
    let mut t = Test::new(false);
    t.key(LMOD4, D);
    t.ext_key(RMOD4, D);
    assert!(t.out.notices.contains(&Notice::StateChanged));
    t.ext_key(RMOD4, U);
    t.key(LMOD4, U);
    assert!(t.engine.mod4_lock());

    // Layer 4 without holding Mod4
    assert!(t.key(KEY_E, D));
    assert_eq!(t.events(), [Vk(vk::RIGHT, D)]);
    t.key(KEY_E, U);

    // Both Mod4 keys again release the lock
    t.key(LMOD4, D);
    t.ext_key(RMOD4, D);
    t.ext_key(RMOD4, U);
    t.key(LMOD4, U);
    assert!(!t.engine.mod4_lock());

    // Can be disabled (#86)
    let mut t = Test::with_config(false, EngineConfig { enable_mod4_lock: false, ..EngineConfig::default() });
    t.key(LMOD4, D);
    t.ext_key(RMOD4, D);
    assert!(!t.engine.mod4_lock());
}

#[test]
fn switching_layers_releases_held_keys() {
    let mut t = Test::new(false);
    t.key(LMOD4, D);
    t.key(KEY_I, D);
    assert_eq!(t.events(), [Vk(vk::LEFT, D)]);
    // Releasing Mod4 first switches to layer 1, which releases the arrow
    t.key(LMOD4, U);
    assert_eq!(t.events(), [Vk(vk::LEFT, U)]);
    // The later release of the physical key produces nothing more
    assert!(t.key(KEY_I, U));
    assert_eq!(t.events(), []);
}

#[test]
fn m3_f1_and_f10_toggle_osk_and_one_handed_mode() {
    let mut t = Test::new(false);
    t.key(LMOD3, D);
    assert!(t.event(vk::F1, 0x3B, false, D));
    assert_eq!(t.out.notices, [Notice::ToggleOsk]);
    assert!(t.event(vk::F10, 0x44, false, D));
    assert_eq!(t.out.notices, [Notice::ToggleOneHandedMode]);
}

// ─── Fake and injected events ──────────────────────────────────────────────────

#[test]
fn injected_and_unicode_events_pass_untouched() {
    let mut t = Test::new(true);
    t.out.clear();
    let injected = KeyEvent { vk: 0x41, scan: Scancode::new(KEY_A, false), down: D, injected: true };
    assert!(!t.engine.handle(&injected, &mut t.platform, &mut t.out));
    assert!(!t.event(vk::PACKET, 0, false, D));
    assert_eq!(t.events(), []);
}

#[test]
fn fake_lctrl_of_altgr_is_swallowed() {
    let mut t = Test::new(false);
    assert!(t.event(vk::LCONTROL, 0x21D, false, D));
    assert!(t.event(vk::LCONTROL, 0x21D, false, U));
    assert_eq!(t.events(), []);
}

#[test]
fn fake_shift_around_numpad_keys_is_swallowed() {
    let mut t = Test::new(true);
    t.event(vk::LSHIFT, LSHIFT, false, D);
    // Fake Shift up with its special scancode
    assert!(t.event(vk::LSHIFT, 0x22A, false, U));
    t.key(0x4F, D); // numpad 1
    t.key(0x4F, U);
    // The fake Shift down afterwards comes without the special scancode sometimes
    assert!(t.event(vk::LSHIFT, LSHIFT, false, D));
    assert_eq!(t.events(), []);
}

// ─── Standalone mode ───────────────────────────────────────────────────────────

#[test]
fn both_shift_keys_toggle_capslock() {
    let mut t = Test::new(true);
    assert!(t.event(vk::LSHIFT, LSHIFT, false, D));
    assert_eq!(t.events(), [Vk(vk::LSHIFT, D)]);
    assert!(t.event(vk::RSHIFT, RSHIFT, true, D));
    assert_eq!(t.events(), [Vk(vk::CAPITAL, D), Vk(vk::CAPITAL, U), Vk(vk::RSHIFT, D)]);
}

#[test]
fn capslock_selects_layer_2_for_capslockable_keys() {
    let mut t = Test::new(true);
    t.platform.capslock = true;
    // ß has no native key combination in the fake platform, so it's sent as Unicode
    assert_eq!(t.tap(KEY_SZ), [Uni('ẞ', D), Uni('ẞ', U)]);
    // Shift with Capslock goes back to layer 1
    t.event(vk::LSHIFT, LSHIFT, false, D);
    t.key(KEY_SZ, D);
    assert_eq!(t.events(), [Uni('ß', D)]);
}

#[test]
fn native_key_combinations_for_characters() {
    let mut t = Test::new(true);
    t.platform.chars.insert('{', CharKey { vk: 0x37, scan: 0x08, shift: false, altgr: true });
    t.key(LMOD3, D);
    // AltGr as LCtrl+RAlt: LCtrl pressed first and released last
    t.key(KEY_A, D);
    assert_eq!(t.events(), [Vk(vk::LCONTROL, D), Vk(vk::RMENU, D), Vk(0x37, D)]);
    t.key(KEY_A, U);
    assert_eq!(t.events(), [Vk(0x37, U), Vk(vk::RMENU, U), Vk(vk::LCONTROL, U)]);
}

#[test]
fn forced_modifier_order() {
    let mut t = Test::new(true);
    let mut altgr = ForcedModifiers::NONE;
    altgr.set(Modifier::LCTRL, true);
    altgr.set(Modifier::RALT, true);

    t.out.clear();
    t.engine.send_vk_with_modifiers(vk::KEY_Q, Scancode::new(0x10, false), altgr, true, &mut t.out);
    assert_eq!(t.events(), [Vk(vk::LCONTROL, D), Vk(vk::RMENU, D), Vk(vk::KEY_Q, D)]);

    t.out.clear();
    t.engine.send_vk_with_modifiers(vk::KEY_Q, Scancode::new(0x10, false), ForcedModifiers::NONE, false, &mut t.out);
    assert_eq!(t.events(), [Vk(vk::KEY_Q, U), Vk(vk::RMENU, U), Vk(vk::LCONTROL, U)]);
    // Modifiers and key go out in one SendInput batch
    assert_eq!(t.out.batches().count(), 1);
}

#[test]
fn release_all_held_keys_after_lost_input() {
    // After e.g. the lock screen, modifiers we hold are released (#94)
    let mut t = Test::new(true);
    t.event(vk::LSHIFT, LSHIFT, false, D);
    assert!(t.engine.is_modifier_held(Modifier::LSHIFT));

    t.out.clear();
    t.engine.release_all_held_keys(&mut t.platform, &mut t.out);
    assert_eq!(t.events(), [Vk(vk::LSHIFT, U)]);
    assert!(!t.engine.is_modifier_held(Modifier::LSHIFT));
}

#[test]
fn reset_turns_off_capslock_and_kana() {
    let mut t = Test::new(true);
    t.platform.capslock = true;
    t.platform.kana = true;
    t.out.clear();
    t.engine.reset_states(&mut t.platform, &mut t.out);
    assert_eq!(t.events(), [Vk(vk::CAPITAL, D), Vk(vk::CAPITAL, U), Vk(vk::KANA, D), Vk(vk::KANA, U)]);
}

#[test]
fn characters_outside_the_bmp_are_one_surrogate_pair_batch() {
    let keysyms = keysyms();
    let layouts = parse_layouts(r#"{"layouts": [{
        "name": "Emoji", "modifiers": {}, "layers": [{}], "capslockableKeys": [],
        "map": {"10": [{"keysym": "U1F574", "char": "🕴"}]}
    }]}"#, &keysyms).unwrap();
    let mut engine = Engine::new(layouts, keysyms, Compose::from_modules(&[], &[], &reneo_core::keysym::Keysyms::default()), EngineConfig::default());
    engine.set_active_layout(Some(0));
    engine.set_standalone(true);
    let (mut platform, mut out) = (FakePlatform::default(), Output::default());
    engine.handle(&KeyEvent { vk: 0, scan: Scancode::new(0x10, false), down: D, injected: false }, &mut platform, &mut out);
    assert_eq!(out.inputs, [Input::Unicode { unit: 0xD83D, down: D }, Input::Unicode { unit: 0xDD74, down: D }]);
    assert_eq!(out.batches().count(), 1);
}

// ─── One-handed mode ───────────────────────────────────────────────────────────

fn one_handed() -> Test {
    let mut mirror_map = HashMap::new();
    mirror_map.insert(Scancode::new(KEY_A, false), Scancode::new(0x25, false));
    let mut t = Test::with_config(true, EngineConfig { mirror_map, ..EngineConfig::default() });
    t.engine.set_one_handed(true);
    t.platform.chars.insert(' ', CharKey { vk: vk::SPACE, scan: SPACE, shift: false, altgr: false });
    t
}

#[test]
fn one_handed_unused_mirror_key_types_normally() {
    let mut t = one_handed();
    assert!(t.key(SPACE, D));
    assert_eq!(t.events(), []);
    assert!(t.key(SPACE, U));
    assert_eq!(t.events(), [Vk(vk::SPACE, D), Vk(vk::SPACE, U)]);
}

#[test]
fn one_handed_mirrors_keys_tapped_while_held() {
    let mut t = one_handed();
    t.key(SPACE, D);
    assert!(t.key(KEY_A, D));
    assert_eq!(t.events(), []);
    assert!(t.key(KEY_A, U));
    // a is mirrored to the key at 0x25, Neo's r (letters are virtual keys in the Neo layout)
    assert_eq!(t.events(), [Vk(0x52, D), Vk(0x52, U)]);
    assert!(t.key(SPACE, U));
    assert_eq!(t.events(), [], "used mirror key produces no space");
}
