module tests;

// Tests for the key handling logic. Run with "dub test". Instead of calling SendInput, unittest builds
// record all generated input events in reneo.sentInputs (see reneo.sendInputs).
// The tests read keysymdef.h, layouts.json and compose/ from the working directory (the repository root).

version (unittest):

import core.sys.windows.windows;

import std.algorithm : map, canFind;
import std.array : array;
import std.conv : to;
import std.exception : assertThrown, collectException;
import std.file : readText;
import std.json : parseJSON;

import reneo;
import mapping;
import composer;
static import app;

private bool fixturesLoaded;

private void loadFixtures() {
    if (fixturesLoaded) {
        return;
    }
    initKeysyms(".");
    initCompose(".");
    initLayouts(parseJSON(readText("layouts.json"))["layouts"]);
    fixturesLoaded = true;
}

private void setUp(string layoutName, bool standalone) {
    loadFixtures();
    activeLayout = null;
    foreach (ref layout; layouts) {
        if (layout.name == layoutName.to!wstring) {
            activeLayout = &layout;
        }
    }
    assert(activeLayout, "Layout " ~ layoutName ~ " not found");

    standaloneModeActive = standalone;
    oneHandedModeActive = false;
    app.configFilterNeoModifiers = true;
    app.configEnableMod4Lock = true;
    mod4Lock = false;
    forgetHeldKeys();
    sentInputs = [];
}

// Simplified view of a generated keyboard event
private struct Event {
    bool unicode;  // true: code is a UTF-16 code unit, false: code is a VK
    uint code;
    bool down;
}

private Event vk(uint code, bool down) {
    return Event(false, code, down);
}

private Event uni(wchar code, bool down) {
    return Event(true, code, down);
}

private Event[] takeEvents() {
    auto events = sentInputs.map!(i => i.ki.dwFlags & KEYEVENTF_UNICODE
        ? Event(true, i.ki.wScan, !(i.ki.dwFlags & KEYEVENTF_KEYUP))
        : Event(false, i.ki.wVk, !(i.ki.dwFlags & KEYEVENTF_KEYUP))).array;
    sentInputs = [];
    return events;
}

// Feed a physical key event, returns whether the original event is eaten
private bool key(uint scan, bool down, bool extended = false) {
    return handleKeyEvent(Scancode(scan, extended), down);
}

// Scancodes used below
private enum SC_LMOD3 = 0x3A;  // Capslock position
private enum SC_LMOD4 = 0x56;  // ISO key left of Y/Z
private enum SC_RMOD4 = 0x38;  // AltGr, extended
private enum SC_LSHIFT = 0x2A;
private enum SC_TAB = 0x0F;
private enum SC_A = 0x20;  // Neo: a, {, Down
private enum SC_E = 0x21;  // Neo: e, }, Right
private enum SC_I = 0x1F;  // Neo: i, /, Left


// ---- Parsing ----

unittest {
    assert(parseScancode("2A") == Scancode(0x2A, false));
    assert(parseScancode("36+") == Scancode(0x36, true));
    assertThrown(parseScancode("2"));
    assertThrown(parseScancode("2A-"));
    assertThrown(parseScancode("ZZ"));
}

unittest {
    loadFixtures();
    assert(parseKeysym("a") == 0x61);
    assert(parseKeysym("Multi_key") == 0xFF20);
    // Unicode keysyms use legacy values where they exist, otherwise the codepoint plus an offset
    assert(parseKeysym("U20AC") == 0x20AC);
    assert(parseKeysym("U1F574") == 0x0101F574);
    assert(parseKeysym("does_not_exist") == KEYSYM_VOID);
}

unittest {
    loadFixtures();
    auto entry = parseLine(`<Multi_key> <a> <e>  : "æ"  ae # LATIN SMALL LETTER AE`);
    assert(entry.keysyms == [parseKeysym("Multi_key"), parseKeysym("a"), parseKeysym("e")]);
    assert(entry.result == "æ"w);

    auto escaped = parseLine(`<Multi_key> <backslash> <n> : "\n\"\\"`);
    assert(escaped.result == "\n\"\\"w);

    assertThrown(parseLine("# just a comment"));
    assertThrown(parseLine(""));

    // Malformed lines must throw instead of hanging or reading past the end of the line
    assertThrown(parseLine("<Multi_key"));
    assertThrown(parseLine(`<Multi_key> <a> "x"`));
    assertThrown(parseLine(`<Multi_key> <a> : "unterminated`));
    assertThrown(parseLine(`<Multi_key> <a> : "ends with backslash\`));
}

unittest {
    wchar[2] units;
    assert(toUTF16Units('a', units) == 1 && units[0] == 'a');
    assert(toUTF16Units(0x1F574, units) == 2 && units[0] == 0xD83D && units[1] == 0xDD74);
}


// ---- Layouts ----

unittest {
    loadFixtures();
    assert(layouts.length == 8);
    foreach (ref layout; layouts) {
        foreach (entry; layout.map.byValue) {
            assert(entry.layers.length == layout.layers.length);
        }
    }
}

unittest {
    // Every keysym in the shipped layouts must be known, otherwise compose can't use the key
    loadFixtures();
    auto json = parseJSON(readText("layouts.json"));
    string[] unknown;
    foreach (jsonLayout; json["layouts"].array) {
        foreach (string scancode, jsonKeys; jsonLayout["map"]) {
            foreach (jsonKey; jsonKeys.array) {
                if ("keysym" in jsonKey && jsonKey["keysym"].str != "VoidSymbol" && parseKeysym(jsonKey["keysym"].str) == KEYSYM_VOID) {
                    unknown ~= jsonLayout["name"].str ~ "/" ~ scancode ~ ": " ~ jsonKey["keysym"].str;
                }
            }
        }
    }
    assert(unknown.length == 0, unknown.to!string);
}

unittest {
    // Invalid custom layouts must result in a helpful exception instead of a crash (#114)
    auto json = parseJSON(`[{
        "name": "Broken",
        "modifiers": {"2A": "LShift"},
        "layers": [{"Shift": false}, {"Shift": true}],
        "capslockableKeys": ["10", "11"],
        "map": {"10": [{"keysym": "x", "char": "x"}, {"keysym": "X", "char": "X"}]}
    }]`);
    auto e = collectException(initLayouts(json));
    assert(e !is null);
    assert(e.msg.canFind("Broken") && e.msg.canFind("11"), e.msg);

    auto tooFewLayers = parseJSON(`[{
        "name": "Short",
        "modifiers": {},
        "layers": [{"Shift": false}, {"Shift": true}],
        "capslockableKeys": [],
        "map": {"10": [{"keysym": "x", "char": "x"}]}
    }]`);
    e = collectException(initLayouts(tooFewLayers));
    assert(e !is null && e.msg.canFind("Short"), e.msg);

    auto nonBmp = parseJSON(`[{
        "name": "Emoji",
        "modifiers": {},
        "layers": [{}],
        "capslockableKeys": [],
        "map": {"10": [{"keysym": "U1F574", "char": "🕴"}]}
    }]`);
    initLayouts(nonBmp);
    assert(layouts[0].map[Scancode(0x10, false)].layers[0].charCode == 0x1F574);

    // initLayouts replaced the global layouts, reload them for the other tests
    fixturesLoaded = false;
}


// ---- Compose ----

private NeoKey keysymKey(string keysymName) {
    NeoKey nk;
    nk.keysym = parseKeysym(keysymName);
    nk.keytype = NeoKeyType.VKEY;
    return nk;
}

private ComposeResult composeSequence(string[] keysymNames) {
    ComposeResult result;
    foreach (name; keysymNames) {
        result = compose(keysymKey(name));
    }
    return result;
}

unittest {
    loadFixtures();
    resetCompose();

    auto result = composeSequence(["Multi_key", "o", "c"]);
    assert(result.type == ComposeResultType.FINISH);
    assert(result.result == "©"w);

    // Unknown sequences are aborted and the typed characters are output
    result = composeSequence(["Multi_key", "o", "Q"]);
    assert(result.type == ComposeResultType.ABORT);
    assert(result.result == "oQ"w);

    // Escape aborts without output
    result = composeSequence(["Multi_key", "o", "Escape"]);
    assert(result.type == ComposeResultType.ABORT && result.result == ""w);

    // Keys outside of compose sequences pass
    assert(compose(keysymKey("a")).type == ComposeResultType.PASS);
}

unittest {
    loadFixtures();
    resetCompose();
    auto result = composeSequence(["Multi_key", "u", "u", "1", "f", "5", "7", "4", "space"]);
    assert(result.type == ComposeResultType.FINISH);
    assert(result.result == "🕴"w);

    result = composeSequence(["Multi_key", "r", "n", "1", "9", "7", "0", "space"]);
    assert(result.type == ComposeResultType.FINISH);
    assert(result.result == "ⅿⅽⅿⅼⅹⅹ"w);

    result = composeSequence(["Multi_key", "R", "N", "4", "space"]);
    assert(result.result == "ⅠⅤ"w);

    // 0 is not a roman numeral
    result = composeSequence(["Multi_key", "r", "n", "0", "space"]);
    assert(result.type == ComposeResultType.ABORT);
}

unittest {
    loadFixtures();
    // A partial sequence must not survive a reset (used after lost input)
    compose(keysymKey("Multi_key"));
    resetCompose();
    assert(compose(keysymKey("a")).type == ComposeResultType.PASS);
}

unittest {
    // Reloading must not duplicate compose entries
    loadFixtures();
    initCompose(".");
    auto count = addedEntries;
    assert(count > 1000);
    initCompose(".");
    assert(addedEntries == count);
}


// ---- Key handling ----

unittest {
    // Extension mode: Mod4 is filtered and layer 4 produces real navigation keys
    setUp("Neo", false);
    assert(key(SC_LMOD4, true));
    assert(takeEvents() == []);

    assert(key(SC_I, true));
    assert(takeEvents() == [vk(VK_LEFT, true)]);
    assert(key(SC_I, false));
    assert(takeEvents() == [vk(VK_LEFT, false)]);

    assert(key(SC_LMOD4, false));
    assert(takeEvents() == []);

    // Back on layer 1, letters are left to the native layout
    assert(!key(SC_I, true));
    assert(!key(SC_I, false));
    assert(takeEvents() == []);
}

unittest {
    // Extension mode: layer 3 characters are sent as Unicode
    setUp("Neo", false);
    key(SC_LMOD3, true);
    assert(key(SC_A, true));
    assert(takeEvents() == [uni('{', true)]);
    assert(key(SC_A, false));
    assert(takeEvents() == [uni('{', false)]);
    key(SC_LMOD3, false);
}

// Find the physical key that produces a keysym on the first layer of the active layout
private uint scancodeFor(string keysymName) {
    auto keysym = parseKeysym(keysymName);
    foreach (scan, entry; activeLayout.map) {
        if (entry.layers[0].keysym == keysym) {
            return scan.scan;
        }
    }
    assert(false, "No key for " ~ keysymName);
}

unittest {
    // Compose with M3+Tab
    setUp("Neo", false);
    key(SC_LMOD3, true);
    assert(key(SC_TAB, true));
    key(SC_TAB, false);
    key(SC_LMOD3, false);
    assert(takeEvents() == []);

    // Keys of the sequence are eaten, the result is sent at the end
    auto scanO = scancodeFor("o");
    auto scanC = scancodeFor("c");
    assert(key(scanO, true));
    assert(key(scanO, false));
    assert(takeEvents() == []);
    assert(key(scanC, true));
    assert(key(scanC, false));
    assert(takeEvents() == [uni('©', true), uni('©', false)]);
}

unittest {
    // Mod4 lock with both Mod4 keys, and the option to disable it (#86)
    setUp("Neo", false);
    key(SC_LMOD4, true);
    key(SC_RMOD4, true, true);
    key(SC_RMOD4, false, true);
    key(SC_LMOD4, false);
    assert(mod4Lock);

    // Layer 4 without holding Mod4
    assert(key(SC_E, true));
    assert(takeEvents() == [vk(VK_RIGHT, true)]);
    key(SC_E, false);
    takeEvents();

    // Pressing both again releases the lock
    key(SC_LMOD4, true);
    key(SC_RMOD4, true, true);
    key(SC_RMOD4, false, true);
    key(SC_LMOD4, false);
    assert(!mod4Lock);

    setUp("Neo", false);
    app.configEnableMod4Lock = false;
    key(SC_LMOD4, true);
    key(SC_RMOD4, true, true);
    key(SC_RMOD4, false, true);
    key(SC_LMOD4, false);
    assert(!mod4Lock);
}

unittest {
    // Forced modifiers: LCtrl is pressed before RAlt, and released after it
    setUp("Neo", true);
    PartialModifierState altGr = [Modifier.LCTRL: true, Modifier.RALT: true];
    sendVKWithModifiers(VKEY.VK_KEY_Q, Scancode(0x10, false), altGr, true);
    assert(takeEvents() == [vk(VK_LCONTROL, true), vk(VK_RMENU, true), vk(VKEY.VK_KEY_Q, true)]);

    sendVKWithModifiers(VKEY.VK_KEY_Q, Scancode(0x10, false), null, false);
    assert(takeEvents() == [vk(VKEY.VK_KEY_Q, false), vk(VK_RMENU, false), vk(VK_LCONTROL, false)]);
}

unittest {
    // After lost input (e.g. lock screen), modifiers we hold are released (#94)
    setUp("Neo", true);
    assert(key(SC_LSHIFT, true));
    assert(takeEvents() == [vk(VK_LSHIFT, true)]);
    assert(isModifierHeld(Modifier.LSHIFT));

    releaseAllHeldKeys();
    assert(takeEvents() == [vk(VK_LSHIFT, false)]);
    assert(!isModifierHeld(Modifier.LSHIFT));
}

unittest {
    // Characters outside of the BMP are sent as a surrogate pair
    sentInputs = [];
    sendUnicodeChar(0x1F574, true);
    assert(takeEvents() == [uni(0xD83D, true), uni(0xDD74, true)]);
    sendUnicodeChar(0x1F574, false);
    assert(takeEvents() == [uni(0xD83D, false), uni(0xDD74, false)]);
}

unittest {
    // appString returns "" if formatting fails, so make sure the security warning renders in both languages
    import localization : initLocalization, appString, AppString, Language;
    import std.algorithm : canFind;

    foreach (language; [Language.ENGLISH, Language.GERMAN]) {
        initLocalization(language);
        string text = appString(AppString.WARNING_UNPROTECTED_INSTALLATION, `C:\Users\someone\ReNeo`);
        assert(text.canFind(`C:\Users\someone\ReNeo`), text);
        assert(text.canFind(`C:\Program Files\ReNeo`) && text.canFind(`%APPDATA%\ReNeo`), text);
    }
}


// ---- Debug log ----

unittest {
    // Only keys that can't produce text are logged in detail
    setUp("Neo", true);
    assert(isNonTextKey(VKEY.VK_LSHIFT, Scancode(0x2A, false)));
    assert(isNonTextKey(VKEY.VK_F5, Scancode(0x3F, false)));
    assert(isNonTextKey(VKEY.VK_LEFT, Scancode(0x4B, true)));
    assert(isNonTextKey(VKEY.VK_CAPITAL, Scancode(SC_LMOD3, false)));  // Mod3 in the Neo layout

    assert(!isNonTextKey(VKEY.VK_KEY_A, Scancode(0x1E, false)));
    assert(!isNonTextKey(VKEY.VK_KEY_5, Scancode(0x06, false)));
    assert(!isNonTextKey(VKEY.VK_SPACE, Scancode(0x39, false)));
    assert(!isNonTextKey(VKEY.VK_RETURN, Scancode(0x1C, false)));
    assert(!isNonTextKey(VKEY.VK_BACK, Scancode(0x0E, false)));
    assert(!isNonTextKey(VKEY.VK_NUMPAD1, Scancode(0x4F, false)));
    assert(!isNonTextKey(VKEY.VK_OEM_PERIOD, Scancode(0x34, false)));
    assert(!isNonTextKey(VKEY.VK_PACKET, Scancode(0x61, false)));  // Unicode packets carry the character as scancode
}

unittest {
    // Retention: only old ReNeo log files are deleted
    import logging : deleteOldLogFiles;
    import std.file : tempDir, mkdirRecurse, rmdirRecurse, write, exists, setTimes;
    import std.path : buildPath;
    import std.datetime.systime : Clock;
    import core.time : days, hours;

    string dir = buildPath(tempDir, "reneo_retention_test");
    mkdirRecurse(dir);
    scope (exit) rmdirRecurse(dir);

    auto now = Clock.currTime();
    string oldLog = buildPath(dir, "reneo_log_2020-01-01_120000.txt");
    string newLog = buildPath(dir, "reneo_log_2026-01-01_120000.txt");
    string otherFile = buildPath(dir, "notes.txt");
    foreach (file; [oldLog, newLog, otherFile]) {
        write(file, "x");
    }
    setTimes(oldLog, now - 8.days, now - 8.days);
    setTimes(newLog, now - 6.days, now - 6.days);
    setTimes(otherFile, now - 30.days, now - 30.days);

    deleteOldLogFiles(dir, 7, now);
    assert(!exists(oldLog));
    assert(exists(newLog));
    assert(exists(otherFile));
}

unittest {
    import localization : initLocalization, appString, AppString, Language;
    import std.algorithm : canFind;

    foreach (language; [Language.ENGLISH, Language.GERMAN]) {
        initLocalization(language);
        string text = appString(AppString.LOG_CONSENT, `C:\logs`, 7);
        assert(text.canFind(`C:\logs`) && text.canFind("7"), text);
    }
}

version (FileLogging) unittest {
    // The log of the shipped debug build must not contain typed text. Type a "secret" in every way that
    // produces log output (physical keys, standalone key combos and Unicode, compose) and check the log file.
    import logging : startFileLogging, currentLogFilePath, flushLogFile, closeLogFile;
    import std.file : tempDir, readText, rmdirRecurse, exists;
    import std.path : buildPath;
    import std.algorithm : canFind;
    import std.format : format;

    string logDir = buildPath(tempDir, "reneo_log_test");
    if (exists(logDir)) {
        rmdirRecurse(logDir);
    }
    startFileLogging(true, logDir, 7);
    scope (exit) {
        closeLogFile();
        rmdirRecurse(logDir);
    }

    void hookKey(uint vk, uint scan, bool down, bool injected = false) {
        KBDLLHOOKSTRUCT event;
        event.vkCode = vk;
        event.scanCode = scan;
        event.flags = (down ? 0 : LLKHF_UP) | (injected ? LLKHF_INJECTED : 0);
        keyboardHook(down ? WM_KEYDOWN : WM_KEYUP, event);
    }

    // Physical keys in standalone mode, including layer 3 characters that are sent as key combos or Unicode
    setUp("Neo", true);
    foreach (keysym; ["s", "e", "c", "r", "e", "t"]) {
        uint scan = scancodeFor(keysym);
        hookKey(MapVirtualKey(scan, MAPVK_VSC_TO_VK), scan, true);
        hookKey(MapVirtualKey(scan, MAPVK_VSC_TO_VK), scan, false);
    }
    hookKey(VK_CAPITAL, SC_LMOD3, true);
    hookKey(0x44, SC_A, true);  // Neo layer 3: {
    hookKey(0x44, SC_A, false);
    hookKey(VK_CAPITAL, SC_LMOD3, false);

    // Our own injected Unicode packet, which carries the character in the scancode field
    hookKey(VK_PACKET, 'q', true, true);

    // Compose
    setUp("Neo", false);
    hookKey(VK_CAPITAL, SC_LMOD3, true);
    hookKey(VK_TAB, SC_TAB, true);
    hookKey(VK_TAB, SC_TAB, false);
    hookKey(VK_CAPITAL, SC_LMOD3, false);
    foreach (keysym; ["o", "c"]) {
        uint scan = scancodeFor(keysym);
        hookKey(0, scan, true);
        hookKey(0, scan, false);
    }
    sentInputs = [];

    flushLogFile();
    string log = readText(currentLogFilePath());

    assert(log.canFind("text key"), log);
    assert(log.canFind("VK_CAPITAL") || log.canFind("VK_TAB") || log.canFind("(0x14)"), log);  // modifiers stay visible
    assert(!log.canFind("VK_KEY_"), log);
    assert(!log.canFind("VK_PACKET"), log);
    assert(!log.canFind("{"), log);
    assert(!log.canFind("©"), log);
    assert(!log.canFind("Next: "), log);
    foreach (keysym; ["s", "e", "c", "r", "t", "o"]) {
        assert(!log.canFind(format("Scan 0x%04X", scancodeFor(keysym))), log);
    }
    assert(!log.canFind(format("Scan 0x%04X", 'q')), log);
}

unittest {
    // Every tray menu item has an access key ("&"), and they are unique within the menu
    import localization : initLocalization, appString, AppString, Language;
    import std.algorithm : countUntil;
    import std.uni : toLower;

    foreach (language; [Language.ENGLISH, Language.GERMAN]) {
        initLocalization(language);
        foreach (toggle; [AppString.MENU_DISABLE, AppString.MENU_ENABLE]) {
            dchar[] keys;
            foreach (item; [AppString.MENU_CHOOSE_LAYOUT, AppString.MENU_OSK, AppString.MENU_ONE_HANDED_MODE,
                    AppString.MENU_OPEN_SETTINGS, AppString.MENU_OPEN_LOG_FOLDER, AppString.MENU_RELOAD, toggle, AppString.MENU_QUIT]) {
                // Items with a hotkey take it as argument, the others take none
                string text = appString(item, "X");
                if (text.length == 0) {
                    text = appString(item);
                }
                auto i = text.countUntil('&');
                assert(i >= 0 && i + 1 < text.length, text);
                dchar key = text[i + 1 .. $].to!dstring[0].toLower;
                assert(!keys.canFind(key), text);
                keys ~= key;
            }
        }
    }
}
