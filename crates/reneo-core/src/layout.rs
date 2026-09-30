//! Keyboard layouts from layouts.json: which modifier sits where, which modifier combination selects which
//! layer, and what every key produces on every layer.

use std::collections::HashMap;

use serde_json::Value;

use crate::keys::{vk, ForcedModifiers, ModKind, Modifier, Scancode};
use crate::keysym::{Keysyms, KEYSYM_VOID};

// ─── Types ─────────────────────────────────────────────────────────────────────

/// What a key does on one layer
#[derive(Clone, Debug, PartialEq)]
pub enum KeyAction {
    /// Send a virtual key (navigation keys, special keys), optionally forcing native modifiers
    Vk { vk: u16, mods: ForcedModifiers },
    /// Produce a character
    Char(char),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeoKey {
    pub keysym: u32,
    pub action: KeyAction,
    /// Label for the on-screen keyboard
    pub label: String,
}

impl NeoKey {
    /// Unmapped keys
    pub fn void() -> NeoKey {
        NeoKey { keysym: KEYSYM_VOID, action: KeyAction::Vk { vk: vk::VOID, mods: ForcedModifiers::NONE }, label: String::new() }
    }
}

#[derive(Clone, Debug)]
pub struct MapEntry {
    /// One entry per layer
    pub layers: Vec<NeoKey>,
    /// Is this key affected by Capslock (usually letters)
    pub capslockable: bool,
}

/// Required modifier state of a layer: kinds that must be held (`true`) or must not be held (`false`).
/// Kinds that aren't listed don't matter.
pub type LayerRequirement = Vec<(ModKind, bool)>;

#[derive(Clone, Debug)]
pub struct Layout {
    pub name: String,
    /// Name of the native driver DLL, if the layout has one (e.g. "kbdneo2.dll")
    pub dll_name: Option<String>,
    pub modifiers: HashMap<Scancode, Modifier>,
    pub layers: Vec<LayerRequirement>,
    pub map: HashMap<Scancode, MapEntry>,
}

impl Layout {
    /// The key on the given layer (1-based, like everywhere else in ReNeo)
    pub fn key(&self, scan: Scancode, layer: usize) -> Option<&NeoKey> {
        self.map.get(&scan).and_then(|entry| entry.layers.get(layer - 1))
    }

    pub fn is_capslockable(&self, scan: Scancode) -> bool {
        self.map.get(&scan).is_some_and(|entry| entry.capslockable)
    }
}

// ─── Loading ───────────────────────────────────────────────────────────────────

/// Loads all layouts from the content of layouts.json. Errors name the layout and key, so users can find mistakes
/// in their custom layouts.
pub fn parse_layouts(json: &str, keysyms: &Keysyms) -> Result<Vec<Layout>, String> {
    let root: Value = serde_json::from_str(json).map_err(|e| format!("layouts.json: {e}"))?;
    let array = root
        .get("layouts")
        .and_then(Value::as_array)
        .ok_or("layouts.json: \"layouts\" must be an array")?;
    array.iter().map(|layout| parse_layout(layout, keysyms)).collect()
}

fn parse_layout(json: &Value, keysyms: &Keysyms) -> Result<Layout, String> {
    let name = json.get("name").and_then(Value::as_str).ok_or("Every layout needs a \"name\"")?.to_string();
    parse_layout_content(json, keysyms, name.clone()).map_err(|e| format!("Layout '{name}': {e}"))
}

fn parse_layout_content(json: &Value, keysyms: &Keysyms, name: String) -> Result<Layout, String> {
    let dll_name = match json.get("dllName") {
        None => None,
        Some(value) => Some(value.as_str().ok_or("\"dllName\" must be a string")?.to_string()),
    };

    let mut modifiers = HashMap::new();
    for (scancode, modifier) in object(json, "modifiers")? {
        let modifier = modifier.as_str().ok_or_else(|| format!("Modifier of key {scancode} must be a string"))?;
        modifiers.insert(Scancode::parse(scancode)?, Modifier::parse(modifier)?);
    }

    let mut layers = Vec::new();
    for layer in array(json, "layers")? {
        let layer = layer.as_object().ok_or("Every layer must be an object")?;
        let mut requirement = LayerRequirement::new();
        for (modifier, held) in layer {
            let held = held.as_bool().ok_or_else(|| format!("Layer modifier {modifier} must be true or false"))?;
            requirement.push((Modifier::parse(modifier)?.kind, held));
        }
        layers.push(requirement);
    }

    let mut map = HashMap::new();
    for (scancode_text, keys) in object(json, "map")? {
        let scancode = Scancode::parse(scancode_text)?;
        if map.contains_key(&scancode) {
            return Err(format!("Key {scancode_text} is mapped more than once"));
        }
        let keys = keys.as_array().ok_or_else(|| format!("Key {scancode_text} must be an array"))?;
        if keys.len() < layers.len() {
            return Err(format!(
                "Key {scancode_text} has {} entries, but the layout defines {} layers",
                keys.len(),
                layers.len()
            ));
        }
        let layer_keys = keys[..layers.len()]
            .iter()
            .enumerate()
            .map(|(i, key)| parse_key(key, keysyms).map_err(|e| format!("Key {scancode_text}, layer {}: {e}", i + 1)))
            .collect::<Result<Vec<_>, _>>()?;
        map.insert(scancode, MapEntry { layers: layer_keys, capslockable: false });
    }

    for scancode in array(json, "capslockableKeys")? {
        let text = scancode.as_str().ok_or("Capslockable keys must be strings")?;
        let entry = map
            .get_mut(&Scancode::parse(text)?)
            .ok_or_else(|| format!("Capslockable key {text} is not mapped"))?;
        entry.capslockable = true;
    }

    Ok(Layout { name, dll_name, modifiers, layers, map })
}

fn parse_key(json: &Value, keysyms: &Keysyms) -> Result<NeoKey, String> {
    let text = |field: &str| -> Result<Option<&str>, String> {
        match json.get(field) {
            None => Ok(None),
            Some(value) => value.as_str().map(Some).ok_or_else(|| format!("\"{field}\" must be a string")),
        }
    };

    let keysym = text("keysym")?.map_or(KEYSYM_VOID, |name| keysyms.parse_keysym(name));

    let action = if let Some(name) = text("vk")? {
        let code = vk::parse(name).ok_or_else(|| format!("Unknown virtual key '{name}'"))?;
        let mut mods = ForcedModifiers::NONE;
        if let Some(object) = json.get("mods") {
            let object = object.as_object().ok_or("\"mods\" must be an object")?;
            for (modifier, pressed) in object {
                let pressed = pressed.as_bool().ok_or_else(|| format!("Modifier {modifier} must be true or false"))?;
                // Only native modifiers can be forced, others are ignored (like the D version)
                mods.set(Modifier::parse(modifier)?, pressed);
            }
        }
        KeyAction::Vk { vk: code, mods }
    } else if let Some(chars) = text("char")? {
        let mut iter = chars.chars();
        match (iter.next(), iter.next()) {
            (Some(c), None) => KeyAction::Char(c),
            _ => return Err(format!("\"char\" must be exactly one character, got \"{chars}\"")),
        }
    } else {
        NeoKey::void().action
    };

    let label = match (text("label")?, &action) {
        (Some(label), _) => label.to_string(),
        (None, KeyAction::Char(c)) => c.to_string(),
        (None, _) => String::new(),
    };

    Ok(NeoKey { keysym, action, label })
}

fn object<'a>(json: &'a Value, field: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    json.get(field).and_then(Value::as_object).ok_or_else(|| format!("\"{field}\" must be an object"))
}

fn array<'a>(json: &'a Value, field: &str) -> Result<&'a Vec<Value>, String> {
    json.get(field).and_then(Value::as_array).ok_or_else(|| format!("\"{field}\" must be an array"))
}
