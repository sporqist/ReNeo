//! Compose sequences in the XCompose format (`compose/*.module`, minus the entries in `compose/*.remove`) and the
//! special modes for Unicode input (♫uu<hex>␣) and Roman numerals (♫rn<number>␣, ♫RN<number>␣).

use std::fs;
use std::path::Path;

use crate::keysym::Keysyms;
use crate::layout::{KeyAction, NeoKey};

// ─── Parsing ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub struct ComposeEntry {
    pub keysyms: Vec<u32>,
    pub result: String,
}

/// Parses one line like `<Multi_key> <a> <e> : "æ" ae # comment`.
/// Returns `Ok(None)` for empty and comment lines, `Err` for malformed lines.
pub fn parse_line(line: &str, keysyms: &Keysyms) -> Result<Option<ComposeEntry>, String> {
    let mut rest = line.trim_start_matches([' ', '\t']);
    if !rest.starts_with('<') {
        return Ok(None);
    }

    let mut entry_keysyms = Vec::new();
    while let Some(after) = rest.strip_prefix('<') {
        let end = after.find('>').ok_or_else(|| format!("Expected '>' in compose line: {line}"))?;
        entry_keysyms.push(keysyms.parse_keysym(&after[..end]));
        rest = after[end + 1..].trim_start_matches([' ', '\t']);
    }

    rest = rest.strip_prefix(':').ok_or_else(|| format!("Expected ':' in compose line: {line}"))?;
    rest = rest.trim_start_matches([' ', '\t']);
    rest = rest.strip_prefix('"').ok_or_else(|| format!("Expected '\"' in compose line: {line}"))?;

    let mut result = String::new();
    let mut chars = rest.chars();
    loop {
        match chars.next() {
            None => return Err(format!("Expected '\"' in compose line: {line}")),
            Some('"') => break,
            Some('\\') => match chars.next() {
                None => return Err(format!("Expected '\"' in compose line: {line}")),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some(c) => result.push(c),
            },
            Some(c) => result.push(c),
        }
    }

    if result.is_empty() {
        return Err(format!("Empty result in compose line: {line}"));
    }
    Ok(Some(ComposeEntry { keysyms: entry_keysyms, result }))
}

// ─── Tree ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
enum SpecialMode {
    Unicode,
    LowerRoman,
    UpperRoman,
}

#[derive(Debug)]
struct Node {
    keysym: u32,
    children: Vec<usize>,
    result: String,
    special: Option<SpecialMode>,
}

/// All compose sequences as a tree: every path from the root to a leaf is one sequence
#[derive(Debug)]
struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    fn new() -> Tree {
        Tree { nodes: vec![Node { keysym: 0, children: Vec::new(), result: String::new(), special: None }] }
    }

    fn child(&self, node: usize, keysym: u32) -> Option<usize> {
        self.nodes[node].children.iter().copied().find(|&child| self.nodes[child].keysym == keysym)
    }

    /// Adds a sequence. Sequences that extend an existing one, or are a prefix of one, conflict and are skipped.
    fn add(&mut self, keysyms: &[u32], result: &str, special: Option<SpecialMode>) -> bool {
        let mut node = 0;
        for &keysym in keysyms {
            node = match self.child(node, keysym) {
                Some(child) => child,
                None => {
                    if !self.nodes[node].result.is_empty() || self.nodes[node].special.is_some() {
                        return false;
                    }
                    self.nodes.push(Node { keysym, children: Vec::new(), result: String::new(), special: None });
                    let child = self.nodes.len() - 1;
                    self.nodes[node].children.push(child);
                    child
                }
            };
        }
        if !self.nodes[node].children.is_empty() {
            return false;
        }
        self.nodes[node].result = result.to_string();
        self.nodes[node].special = special;
        true
    }

    /// Follows the sequence as far as it exists and compares the result there (same semantics as the D version)
    fn contains(&self, entry: &ComposeEntry) -> bool {
        let mut node = 0;
        for &keysym in &entry.keysyms {
            match self.child(node, keysym) {
                Some(child) => node = child,
                None => break,
            }
        }
        self.nodes[node].result == entry.result
    }
}

// ─── Composer ──────────────────────────────────────────────────────────────────

/// What to do with a key event after compose looked at it
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposeResult {
    /// Not part of a compose sequence, handle the key normally
    Pass,
    /// Part of a sequence, swallow the key
    Eat,
    /// Sequence complete, send [`Compose::output`] instead of the key
    Finish,
    /// Sequence aborted, send [`Compose::output`] (the typed characters, or nothing after Escape)
    Abort,
}

const ROMAN_DIGITS: [[[&str; 2]; 9]; 4] = [
    [["ⅰ", "Ⅰ"], ["ⅰⅰ", "ⅠⅠ"], ["ⅰⅰⅰ", "ⅠⅠⅠ"], ["ⅰⅴ", "ⅠⅤ"], ["ⅴ", "Ⅴ"], ["ⅴⅰ", "ⅤⅠ"], ["ⅴⅰⅰ", "ⅤⅠⅠ"], ["ⅴⅰⅰⅰ", "ⅤⅠⅠⅠ"], ["ⅰⅹ", "ⅠⅩ"]],
    [["ⅹ", "Ⅹ"], ["ⅹⅹ", "ⅩⅩ"], ["ⅹⅹⅹ", "ⅩⅩⅩ"], ["ⅹⅼ", "ⅩⅬ"], ["ⅼ", "Ⅼ"], ["ⅼⅹ", "ⅬⅩ"], ["ⅼⅹⅹ", "ⅬⅩⅩ"], ["ⅼⅹⅹⅹ", "ⅬⅩⅩⅩ"], ["ⅹⅽ", "ⅩⅭ"]],
    [["ⅽ", "Ⅽ"], ["ⅽⅽ", "ⅭⅭ"], ["ⅽⅽⅽ", "ⅭⅭⅭ"], ["ⅽⅾ", "ⅭⅮ"], ["ⅾ", "Ⅾ"], ["ⅾⅽ", "ⅮⅭ"], ["ⅾⅽⅽ", "ⅮⅭⅭ"], ["ⅾⅽⅽⅽ", "ⅮⅭⅭⅭ"], ["ⅽⅿ", "ⅭⅯ"]],
    [["ⅿ", "Ⅿ"], ["ⅿⅿ", "ⅯⅯ"], ["ⅿⅿⅿ", "ⅯⅯⅯ"], ["", ""], ["", ""], ["", ""], ["", ""], ["", ""], ["", ""]],
];

/// Keysyms the special modes need
#[derive(Debug, Default)]
struct SpecialKeysyms {
    space: u32,
    digit_0: u32,
    keypad_0: u32,
    lower_a: u32,
    upper_a: u32,
    escape: u32,
}

#[derive(Debug)]
pub struct Compose {
    tree: Tree,
    keys: SpecialKeysyms,
    /// Number of loaded entries, including ones skipped because of conflicts (like the D version)
    pub added_entries: usize,

    active: bool,
    node: usize,
    sequence: String,
    special: Option<SpecialMode>,
    special_input: String,
    output: String,
}

impl Compose {
    /// Loads all `*.remove` files, then all `*.module` files of the directory, in alphabetical order.
    /// A missing directory gives an empty compose table.
    pub fn load(dir: &Path, keysyms: &Keysyms) -> std::io::Result<Compose> {
        let mut removes = Vec::new();
        let mut modules = Vec::new();
        if dir.is_dir() {
            let mut paths: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).map(|e| e.path()).collect();
            paths.sort();
            for path in paths.iter().filter(|p| p.is_file()) {
                match path.extension().and_then(|e| e.to_str()) {
                    Some("remove") => removes.push(fs::read_to_string(path)?),
                    Some("module") => modules.push(fs::read_to_string(path)?),
                    _ => {}
                }
            }
        }
        let removes: Vec<&str> = removes.iter().map(String::as_str).collect();
        let modules: Vec<&str> = modules.iter().map(String::as_str).collect();
        Ok(Compose::from_modules(&removes, &modules, keysyms))
    }

    /// Builds the compose table from the contents of remove and module files
    pub fn from_modules(removes: &[&str], modules: &[&str], keysyms: &Keysyms) -> Compose {
        let mut removed = Tree::new();
        for line in removes.iter().flat_map(|text| text.lines()) {
            if let Ok(Some(entry)) = parse_line(line, keysyms) {
                removed.add(&entry.keysyms, &entry.result, None);
            }
        }

        let mut tree = Tree::new();
        let mut added_entries = 0;
        for line in modules.iter().flat_map(|text| text.lines()) {
            if let Ok(Some(entry)) = parse_line(line, keysyms) {
                if !removed.contains(&entry) {
                    tree.add(&entry.keysyms, &entry.result, None);
                    added_entries += 1;
                }
            }
        }

        let multi_key = keysyms.parse_keysym("Multi_key");
        let k = |name: &str| keysyms.parse_keysym(name);
        tree.add(&[multi_key, k("u"), k("u")], "", Some(SpecialMode::Unicode));
        tree.add(&[multi_key, k("r"), k("n")], "", Some(SpecialMode::LowerRoman));
        tree.add(&[multi_key, k("R"), k("N")], "", Some(SpecialMode::UpperRoman));

        Compose {
            tree,
            keys: SpecialKeysyms {
                space: k("space"),
                digit_0: k("0"),
                keypad_0: k("KP_0"),
                lower_a: k("a"),
                upper_a: k("A"),
                escape: k("Escape"),
            },
            added_entries,
            active: false,
            node: 0,
            sequence: String::with_capacity(64),
            special: None,
            special_input: String::with_capacity(8),
            output: String::with_capacity(64),
        }
    }

    /// Text to send after [`ComposeResult::Finish`] or [`ComposeResult::Abort`]
    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Aborts any sequence in progress without output
    pub fn reset(&mut self) {
        self.active = false;
        self.special = None;
        self.special_input.clear();
    }

    /// Feeds one key press into the compose state machine
    pub fn compose(&mut self, key: &NeoKey, keysyms: &Keysyms) -> ComposeResult {
        if !self.active {
            if self.tree.child(0, key.keysym).is_none() {
                return ComposeResult::Pass;
            }
            self.active = true;
            self.node = 0;
            self.sequence.clear();
        }

        if let Some(mode) = self.special {
            let result = self.special_key(mode, key.keysym);
            if matches!(result, ComposeResult::Finish | ComposeResult::Abort) {
                self.active = false;
                self.special = None;
            }
            return result;
        }

        // Collect the typed characters, they are the output if the sequence is aborted
        let typed = keysyms.to_char(key.keysym).or(match key.action {
            KeyAction::Char(c) => Some(c),
            _ => None,
        });
        if let Some(c) = typed {
            self.sequence.push(c);
        }

        match self.tree.child(self.node, key.keysym) {
            Some(next) if self.tree.nodes[next].children.is_empty() => {
                if let Some(mode) = self.tree.nodes[next].special {
                    // Leader of a special mode, the following keys go to the special mode
                    self.special = Some(mode);
                    self.special_input.clear();
                    ComposeResult::Eat
                } else {
                    self.active = false;
                    self.output.clear();
                    self.output.push_str(&self.tree.nodes[next].result);
                    ComposeResult::Finish
                }
            }
            Some(next) => {
                self.node = next;
                ComposeResult::Eat
            }
            None => {
                self.active = false;
                self.output.clear();
                if key.keysym != self.keys.escape {
                    self.output.push_str(&self.sequence);
                }
                ComposeResult::Abort
            }
        }
    }

    fn special_key(&mut self, mode: SpecialMode, keysym: u32) -> ComposeResult {
        let keys = &self.keys;
        let in_range = |base: u32, count: u32| keysym >= base && keysym < base + count;
        self.output.clear();

        match mode {
            SpecialMode::Unicode => {
                // Up to six hex digits, finished with space
                let digit = if in_range(keys.digit_0, 10) {
                    Some(char::from(b'0' + (keysym - keys.digit_0) as u8))
                } else if in_range(keys.keypad_0, 10) {
                    Some(char::from(b'0' + (keysym - keys.keypad_0) as u8))
                } else if in_range(keys.lower_a, 6) {
                    Some(char::from(b'a' + (keysym - keys.lower_a) as u8))
                } else if in_range(keys.upper_a, 6) {
                    Some(char::from(b'a' + (keysym - keys.upper_a) as u8))
                } else {
                    None
                };

                if let (Some(digit), true) = (digit, self.special_input.len() < 6) {
                    self.special_input.push(digit);
                    return ComposeResult::Eat;
                }
                let result = if keysym == keys.space && self.special_input.len() >= 2 {
                    match u32::from_str_radix(&self.special_input, 16).ok().filter(|&c| c >= 0x20).and_then(char::from_u32) {
                        Some(c) => {
                            self.output.push(c);
                            ComposeResult::Finish
                        }
                        None => ComposeResult::Abort,
                    }
                } else {
                    ComposeResult::Abort
                };
                self.special_input.clear();
                result
            }
            SpecialMode::LowerRoman | SpecialMode::UpperRoman => {
                // One to four decimal digits (1 to 3999), finished with space
                let digit = if in_range(keys.digit_0, 10) {
                    Some(char::from(b'0' + (keysym - keys.digit_0) as u8))
                } else if in_range(keys.keypad_0, 10) {
                    Some(char::from(b'0' + (keysym - keys.keypad_0) as u8))
                } else {
                    None
                };

                if let (Some(digit), true) = (digit, self.special_input.len() < 4) {
                    self.special_input.push(digit);
                    return ComposeResult::Eat;
                }
                let result = if keysym == keys.space && !self.special_input.is_empty() {
                    match self.special_input.parse::<usize>() {
                        Ok(number @ 1..=3999) => {
                            let case = usize::from(mode == SpecialMode::UpperRoman);
                            for (position, divisor) in [(3, 1000), (2, 100), (1, 10), (0, 1)] {
                                let digit = number / divisor % 10;
                                if digit > 0 {
                                    self.output.push_str(ROMAN_DIGITS[position][digit - 1][case]);
                                }
                            }
                            ComposeResult::Finish
                        }
                        _ => ComposeResult::Abort,
                    }
                } else {
                    ComposeResult::Abort
                };
                self.special_input.clear();
                result
            }
        }
    }
}
