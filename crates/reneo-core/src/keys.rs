//! Physical keys (scancodes), modifiers and Windows virtual-key codes.
//!
//! These are plain values without any Windows dependency, so the engine and its tests run everywhere.

use std::fmt;

// ─── Scancodes ─────────────────────────────────────────────────────────────────

/// Physical position of a key. Some keys only differ in the extended bit (e.g. left and right Ctrl).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Scancode {
    pub code: u16,
    pub extended: bool,
}

impl Scancode {
    pub const fn new(code: u16, extended: bool) -> Self {
        Scancode { code, extended }
    }

    /// Parses the notation of layouts.json: a byte in hex, `+` marks the extended bit ("2A", "36+").
    pub fn parse(text: &str) -> Result<Scancode, String> {
        let (hex, extended) = match text.strip_suffix('+') {
            Some(hex) => (hex, true),
            None => (text, false),
        };
        if hex.len() != 2 {
            return Err(format!("Invalid scancode '{text}', expected two hex digits optionally followed by '+'"));
        }
        let code = u16::from_str_radix(hex, 16)
            .map_err(|_| format!("Invalid scancode '{text}', expected two hex digits optionally followed by '+'"))?;
        Ok(Scancode { code, extended })
    }
}

impl fmt::Display for Scancode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{}", self.code, if self.extended { "+" } else { "" })
    }
}

// ─── Modifiers ─────────────────────────────────────────────────────────────────

/// The kind of a modifier, independent of the side it's on. Layer definitions only name kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModKind {
    Shift,
    Ctrl,
    Alt,
    Mod3,
    Mod4,
    Mod5,
    Mod6,
    Mod7,
    Mod8,
    Mod9,
}

impl ModKind {
    /// Shift, Ctrl and Alt exist in Windows, the others only inside ReNeo
    pub fn is_native(self) -> bool {
        matches!(self, ModKind::Shift | ModKind::Ctrl | ModKind::Alt)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

/// A modifier on a specific side. Mod5 to Mod9 have no natural left and right position, they always use `Left`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Modifier {
    pub kind: ModKind,
    pub side: Side,
}

impl Modifier {
    pub const LSHIFT: Modifier = Modifier { kind: ModKind::Shift, side: Side::Left };
    pub const RSHIFT: Modifier = Modifier { kind: ModKind::Shift, side: Side::Right };
    pub const LCTRL: Modifier = Modifier { kind: ModKind::Ctrl, side: Side::Left };
    pub const RCTRL: Modifier = Modifier { kind: ModKind::Ctrl, side: Side::Right };
    pub const LALT: Modifier = Modifier { kind: ModKind::Alt, side: Side::Left };
    pub const RALT: Modifier = Modifier { kind: ModKind::Alt, side: Side::Right };
    pub const LMOD3: Modifier = Modifier { kind: ModKind::Mod3, side: Side::Left };
    pub const RMOD3: Modifier = Modifier { kind: ModKind::Mod3, side: Side::Right };
    pub const LMOD4: Modifier = Modifier { kind: ModKind::Mod4, side: Side::Left };
    pub const RMOD4: Modifier = Modifier { kind: ModKind::Mod4, side: Side::Right };

    /// Parses modifier names as used in layouts.json, case insensitive: "Shift", "LShift", "RMod4", "Mod5", …
    /// A name without side means the left one.
    pub fn parse(name: &str) -> Result<Modifier, String> {
        let upper = name.to_ascii_uppercase();
        let (side, rest) = match upper.as_bytes().first() {
            Some(b'L') => (Side::Left, &upper[1..]),
            Some(b'R') => (Side::Right, &upper[1..]),
            _ => (Side::Left, upper.as_str()),
        };
        let kind = match rest {
            "SHIFT" => ModKind::Shift,
            "CTRL" => ModKind::Ctrl,
            "ALT" => ModKind::Alt,
            "MOD3" => ModKind::Mod3,
            "MOD4" => ModKind::Mod4,
            "MOD5" => ModKind::Mod5,
            "MOD6" => ModKind::Mod6,
            "MOD7" => ModKind::Mod7,
            "MOD8" => ModKind::Mod8,
            "MOD9" => ModKind::Mod9,
            _ => return Err(format!("Unknown modifier '{name}'")),
        };
        let side = if matches!(kind, ModKind::Mod5 | ModKind::Mod6 | ModKind::Mod7 | ModKind::Mod8 | ModKind::Mod9) {
            if side == Side::Right {
                return Err(format!("Unknown modifier '{name}'"));
            }
            Side::Left
        } else {
            side
        };
        Ok(Modifier { kind, side })
    }

    /// Index into arrays of the six native modifiers (see [`NATIVE_MODIFIERS`]), `None` for Neo modifiers
    pub fn native_index(self) -> Option<usize> {
        let base = match self.kind {
            ModKind::Shift => 0,
            ModKind::Ctrl => 2,
            ModKind::Alt => 4,
            _ => return None,
        };
        Some(base + if self.side == Side::Right { 1 } else { 0 })
    }

    /// Windows virtual-key code of a native modifier
    pub fn vk(self) -> Option<u16> {
        self.native_index().map(|i| vk::LSHIFT + i as u16)
    }

    /// Scancode Windows uses for a native modifier
    pub fn scancode(self) -> Option<Scancode> {
        Some(match self.native_index()? {
            0 => Scancode::new(0x2A, false),
            1 => Scancode::new(0x36, false),
            2 => Scancode::new(0x1D, false),
            3 => Scancode::new(0x1D, true),
            4 => Scancode::new(0x38, false),
            _ => Scancode::new(0x38, true),
        })
    }
}

/// The native modifiers in the order of [`Modifier::native_index`]
pub const NATIVE_MODIFIERS: [Modifier; 6] =
    [Modifier::LSHIFT, Modifier::RSHIFT, Modifier::LCTRL, Modifier::RCTRL, Modifier::LALT, Modifier::RALT];

/// Desired state for some of the native modifiers: `Some(true)` pressed, `Some(false)` released, `None` untouched.
/// Fixed size and `Copy`, so it can be passed around in the hook without allocating.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForcedModifiers(pub [Option<bool>; 6]);

impl ForcedModifiers {
    pub const NONE: ForcedModifiers = ForcedModifiers([None; 6]);

    pub fn get(&self, modifier: Modifier) -> Option<bool> {
        modifier.native_index().and_then(|i| self.0[i])
    }

    pub fn set(&mut self, modifier: Modifier, pressed: bool) {
        if let Some(i) = modifier.native_index() {
            self.0[i] = Some(pressed);
        }
    }
}

// ─── Virtual-key codes ─────────────────────────────────────────────────────────

/// Windows virtual-key codes, see <https://learn.microsoft.com/windows/win32/inputdev/virtual-key-codes>
pub mod vk {
    pub const LBUTTON: u16 = 0x01;
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12;
    pub const CAPITAL: u16 = 0x14;
    pub const KANA: u16 = 0x15;
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21;
    pub const NEXT: u16 = 0x22;
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const KEY_A: u16 = 0x41;
    pub const KEY_Q: u16 = 0x51;
    pub const DIVIDE: u16 = 0x6F;
    pub const F1: u16 = 0x70;
    pub const F10: u16 = 0x79;
    pub const NUMLOCK: u16 = 0x90;
    pub const LSHIFT: u16 = 0xA0;
    pub const RSHIFT: u16 = 0xA1;
    pub const LCONTROL: u16 = 0xA2;
    pub const RCONTROL: u16 = 0xA3;
    pub const LMENU: u16 = 0xA4;
    pub const RMENU: u16 = 0xA5;
    pub const PACKET: u16 = 0xE7;
    pub const VOID: u16 = 0xFF;

    /// Names as used in layouts.json ("VK_LEFT") without the "VK_" prefix
    pub const NAMES: &[(&str, u16)] = &[
        ("LBUTTON", 0x01), ("RBUTTON", 0x02), ("CANCEL", 0x03), ("MBUTTON", 0x04),
        ("XBUTTON1", 0x05), ("XBUTTON2", 0x06), ("BACK", 0x08), ("TAB", 0x09),
        ("CLEAR", 0x0C), ("RETURN", 0x0D), ("SHIFT", 0x10), ("CONTROL", 0x11),
        ("MENU", 0x12), ("PAUSE", 0x13), ("CAPITAL", 0x14), ("KANA", 0x15),
        ("HANGEUL", 0x15), ("HANGUL", 0x15), ("JUNJA", 0x17), ("FINAL", 0x18),
        ("HANJA", 0x19), ("KANJI", 0x19), ("ESCAPE", 0x1B), ("CONVERT", 0x1C),
        ("NONCONVERT", 0x1D), ("ACCEPT", 0x1E), ("MODECHANGE", 0x1F), ("SPACE", 0x20),
        ("PRIOR", 0x21), ("NEXT", 0x22), ("END", 0x23), ("HOME", 0x24),
        ("LEFT", 0x25), ("UP", 0x26), ("RIGHT", 0x27), ("DOWN", 0x28),
        ("SELECT", 0x29), ("PRINT", 0x2A), ("EXECUTE", 0x2B), ("SNAPSHOT", 0x2C),
        ("INSERT", 0x2D), ("DELETE", 0x2E), ("HELP", 0x2F), ("KEY_0", 0x30),
        ("KEY_1", 0x31), ("KEY_2", 0x32), ("KEY_3", 0x33), ("KEY_4", 0x34),
        ("KEY_5", 0x35), ("KEY_6", 0x36), ("KEY_7", 0x37), ("KEY_8", 0x38),
        ("KEY_9", 0x39), ("KEY_A", 0x41), ("KEY_B", 0x42), ("KEY_C", 0x43),
        ("KEY_D", 0x44), ("KEY_E", 0x45), ("KEY_F", 0x46), ("KEY_G", 0x47),
        ("KEY_H", 0x48), ("KEY_I", 0x49), ("KEY_J", 0x4A), ("KEY_K", 0x4B),
        ("KEY_L", 0x4C), ("KEY_M", 0x4D), ("KEY_N", 0x4E), ("KEY_O", 0x4F),
        ("KEY_P", 0x50), ("KEY_Q", 0x51), ("KEY_R", 0x52), ("KEY_S", 0x53),
        ("KEY_T", 0x54), ("KEY_U", 0x55), ("KEY_V", 0x56), ("KEY_W", 0x57),
        ("KEY_X", 0x58), ("KEY_Y", 0x59), ("KEY_Z", 0x5A), ("LWIN", 0x5B),
        ("RWIN", 0x5C), ("APPS", 0x5D), ("SLEEP", 0x5F), ("NUMPAD0", 0x60),
        ("NUMPAD1", 0x61), ("NUMPAD2", 0x62), ("NUMPAD3", 0x63), ("NUMPAD4", 0x64),
        ("NUMPAD5", 0x65), ("NUMPAD6", 0x66), ("NUMPAD7", 0x67), ("NUMPAD8", 0x68),
        ("NUMPAD9", 0x69), ("MULTIPLY", 0x6A), ("ADD", 0x6B), ("SEPARATOR", 0x6C),
        ("SUBTRACT", 0x6D), ("DECIMAL", 0x6E), ("DIVIDE", 0x6F), ("F1", 0x70),
        ("F2", 0x71), ("F3", 0x72), ("F4", 0x73), ("F5", 0x74),
        ("F6", 0x75), ("F7", 0x76), ("F8", 0x77), ("F9", 0x78),
        ("F10", 0x79), ("F11", 0x7A), ("F12", 0x7B), ("F13", 0x7C),
        ("F14", 0x7D), ("F15", 0x7E), ("F16", 0x7F), ("F17", 0x80),
        ("F18", 0x81), ("F19", 0x82), ("F20", 0x83), ("F21", 0x84),
        ("F22", 0x85), ("F23", 0x86), ("F24", 0x87), ("UNDO", 0x89),
        ("NUMLOCK", 0x90), ("SCROLL", 0x91), ("LSHIFT", 0xA0), ("RSHIFT", 0xA1),
        ("LCONTROL", 0xA2), ("RCONTROL", 0xA3), ("LMENU", 0xA4), ("RMENU", 0xA5),
        ("BROWSER_BACK", 0xA6), ("BROWSER_FORWARD", 0xA7), ("BROWSER_REFRESH", 0xA8), ("BROWSER_STOP", 0xA9),
        ("BROWSER_SEARCH", 0xAA), ("BROWSER_FAVORITES", 0xAB), ("BROWSER_HOME", 0xAC), ("VOLUME_MUTE", 0xAD),
        ("VOLUME_DOWN", 0xAE), ("VOLUME_UP", 0xAF), ("MEDIA_NEXT_TRACK", 0xB0), ("MEDIA_PREV_TRACK", 0xB1),
        ("MEDIA_STOP", 0xB2), ("MEDIA_PLAY_PAUSE", 0xB3), ("LAUNCH_MAIL", 0xB4), ("LAUNCH_MEDIA_SELECT", 0xB5),
        ("LAUNCH_APP1", 0xB6), ("LAUNCH_APP2", 0xB7), ("OEM_1", 0xBA), ("OEM_PLUS", 0xBB),
        ("OEM_COMMA", 0xBC), ("OEM_MINUS", 0xBD), ("OEM_PERIOD", 0xBE), ("OEM_2", 0xBF),
        ("OEM_3", 0xC0), ("OEM_4", 0xDB), ("OEM_5", 0xDC), ("OEM_6", 0xDD),
        ("OEM_7", 0xDE), ("OEM_8", 0xDF), ("OEM_102", 0xE2), ("PROCESSKEY", 0xE5),
        ("PACKET", 0xE7), ("ATTN", 0xF6), ("CRSEL", 0xF7), ("EXSEL", 0xF8),
        ("EREOF", 0xF9), ("PLAY", 0xFA), ("ZOOM", 0xFB), ("NONAME", 0xFC),
        ("PA1", 0xFD), ("OEM_CLEAR", 0xFE), ("VOID", 0xFF),
    ];

    /// Looks up a name like "VK_LEFT" (the prefix is required, as in layouts.json and config.json)
    pub fn parse(name: &str) -> Option<u16> {
        let name = name.strip_prefix("VK_")?;
        NAMES.iter().find(|(n, _)| *n == name).map(|&(_, code)| code)
    }

    /// Name without the "VK_" prefix, e.g. for hotkey labels in the tray menu
    pub fn name(code: u16) -> Option<&'static str> {
        NAMES.iter().find(|&&(_, c)| c == code).map(|&(n, _)| n)
    }

    /// Keys that must be sent with the extended flag, otherwise they don't work correctly together with Shift
    pub fn is_extended(code: u16) -> bool {
        matches!(code, INSERT | DELETE | HOME | END | PRIOR | NEXT | UP | DOWN | LEFT | RIGHT | DIVIDE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scancodes() {
        assert_eq!(Scancode::parse("2A"), Ok(Scancode::new(0x2A, false)));
        assert_eq!(Scancode::parse("36+"), Ok(Scancode::new(0x36, true)));
        assert!(Scancode::parse("2").is_err());
        assert!(Scancode::parse("2A-").is_err());
        assert!(Scancode::parse("ZZ").is_err());
        assert!(Scancode::parse("123").is_err());
        assert_eq!(Scancode::new(0x38, true).to_string(), "38+");
    }

    #[test]
    fn parses_modifiers() {
        assert_eq!(Modifier::parse("Shift"), Ok(Modifier::LSHIFT));
        assert_eq!(Modifier::parse("RShift"), Ok(Modifier::RSHIFT));
        assert_eq!(Modifier::parse("lctrl"), Ok(Modifier::LCTRL));
        assert_eq!(Modifier::parse("RMod4"), Ok(Modifier::RMOD4));
        assert_eq!(Modifier::parse("Mod5"), Ok(Modifier { kind: ModKind::Mod5, side: Side::Left }));
        assert!(Modifier::parse("RMod5").is_err());
        assert!(Modifier::parse("Hyper").is_err());

        assert_eq!(Modifier::RALT.vk(), Some(vk::RMENU));
        assert_eq!(Modifier::LMOD3.vk(), None);
        assert_eq!(Modifier::RCTRL.scancode(), Some(Scancode::new(0x1D, true)));
    }

    #[test]
    fn parses_vk_names() {
        assert_eq!(vk::parse("VK_LEFT"), Some(vk::LEFT));
        assert_eq!(vk::parse("VK_KEY_A"), Some(0x41));
        assert_eq!(vk::parse("LEFT"), None);
        assert_eq!(vk::name(0x70), Some("F1"));
    }
}
