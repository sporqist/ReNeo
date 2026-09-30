module localization;

import std.conv : to;
import std.format : format;
import std.utf : toUTF16z;
import std.string : capitalize;

import core.sys.windows.windows : LPCWSTR;
import core.sys.windows.winuser : MOD_WIN, MOD_ALT, MOD_SHIFT, MOD_CONTROL;

import app : HotkeyConfig;
import mapping : VKEY;

enum AppString {
    MENU_DISABLE,
    MENU_ENABLE,
    MENU_RELOAD,
    MENU_CHOOSE_LAYOUT,
    MENU_QUIT,
    MENU_OSK,
    MENU_ONE_HANDED_MODE,

    TRAY_INACTIVE,
    TRAY_LOGGING,

    LOG_CONSENT_TITLE,
    LOG_CONSENT,

    WARNING_SECURITY,
    WARNING_UNPROTECTED_INSTALLATION,

    ERROR_INVALID_HOTKEY_MODIFIER,
    ERROR_BLACKLIST_MUST_CONTAIN_WINDOW_TITLE,
    ERROR_ERROR_OCCURRED_WHILE_STARTING,
    ERROR_WHILE_INITIALIZING,
    ERROR_PATH_DOES_NOT_EXIST,
    ERROR_WHILE_PARSING
}

enum Language {
    ENGLISH,
    GERMAN
}

private Language selectedLanguage;

private string[Language][AppString] stringMap;

void initLocalization(Language lang) {
    stringMap = [
        AppString.MENU_DISABLE: [Language.ENGLISH: "Deactivate\t%s", Language.GERMAN: "Deaktivieren\t%s"],
        AppString.MENU_ENABLE: [Language.ENGLISH: "Activate\t%s", Language.GERMAN: "Aktivieren\t%s"],
        AppString.MENU_RELOAD: [Language.ENGLISH: "Reload", Language.GERMAN: "Neu laden"],
        AppString.MENU_CHOOSE_LAYOUT: [Language.ENGLISH: "Layout", Language.GERMAN: "Tastaturlayout"],
        AppString.MENU_QUIT: [Language.ENGLISH: "Quit", Language.GERMAN: "Beenden"],
        AppString.MENU_OSK: [Language.ENGLISH: "On-Screen Keyboard\t%s", Language.GERMAN: "Bildschirmtastatur\t%s"],
        AppString.MENU_ONE_HANDED_MODE: [Language.ENGLISH: "One-Handed Mode\t%s", Language.GERMAN: "Einhandmodus\t%s"],
        AppString.TRAY_INACTIVE: [Language.ENGLISH: "inactive", Language.GERMAN: "inaktiv"],
        AppString.TRAY_LOGGING: [Language.ENGLISH: " - writing log", Language.GERMAN: " - schreibt Log"],
        AppString.LOG_CONSENT_TITLE: [Language.ENGLISH: "ReNeo debug version", Language.GERMAN: "ReNeo-Debugversion"],
        AppString.LOG_CONSENT: [
            Language.ENGLISH: "This is the debug version of ReNeo. It can write a diagnostic log for bug reports to:\n%s\n\n"
                ~ "The log is meant to be safe to share: keys that produce text, typed characters, compose input and "
                ~ "window titles are replaced by placeholders. It still shows when and how many keys you pressed, "
                ~ "and which modifiers and special keys you used.\n\n"
                ~ "Log files are deleted after %d days.\n\nWrite a log?",
            Language.GERMAN: "Dies ist die Debugversion von ReNeo. Sie kann für Fehlerberichte ein Diagnoseprotokoll schreiben nach:\n%s\n\n"
                ~ "Das Protokoll ist zum Teilen gedacht: Tasten, die Text erzeugen, getippte Zeichen, Compose-Eingaben und "
                ~ "Fenstertitel werden durch Platzhalter ersetzt. Es zeigt aber weiterhin, wann und wie viele Tasten du gedrückt hast "
                ~ "und welche Modifier und Sondertasten du benutzt hast.\n\n"
                ~ "Protokolldateien werden nach %d Tagen gelöscht.\n\nProtokoll schreiben?"
        ],
        AppString.WARNING_SECURITY: [Language.ENGLISH: "ReNeo security warning", Language.GERMAN: "ReNeo-Sicherheitswarnung"],
        AppString.WARNING_UNPROTECTED_INSTALLATION: [
            Language.ENGLISH: "ReNeo runs with administrator rights, but its files can be changed without administrator rights:\n%s\n\n"
                ~ "Any program you run could replace ReNeo's files or layouts and gain administrator rights at your next logon.\n\n"
                ~ `Move ReNeo to a folder that only administrators can change, e.g. C:\Program Files\ReNeo. `
                ~ `Your settings are then stored in %%APPDATA%%\ReNeo.` ~ "\n\n"
                ~ "If you accept the risk, set \"warnUnprotectedInstallation\" to false in config.json.",
            Language.GERMAN: "ReNeo läuft mit Administratorrechten, aber seine Dateien können ohne Administratorrechte verändert werden:\n%s\n\n"
                ~ "Jedes Programm, das du startest, könnte ReNeos Dateien oder Layouts austauschen und bei der nächsten Anmeldung Administratorrechte erlangen.\n\n"
                ~ `Verschiebe ReNeo in einen Ordner, den nur Administratoren ändern können, z. B. C:\Program Files\ReNeo. `
                ~ `Deine Einstellungen liegen dann in %%APPDATA%%\ReNeo.` ~ "\n\n"
                ~ "Wenn du das Risiko in Kauf nimmst, setze \"warnUnprotectedInstallation\" in config.json auf false."
        ],
        AppString.ERROR_INVALID_HOTKEY_MODIFIER: [
            Language.ENGLISH: "Non-existent hotkey modifier '%s'. Possible values are Shift, Ctrl, Alt, Win.",
            Language.GERMAN: "Nicht existierender Hotkey-Modifier '%s'. Mögliche Werte sind Shift, Ctrl, Alt, Win."
        ],
        AppString.ERROR_BLACKLIST_MUST_CONTAIN_WINDOW_TITLE: [
            Language.ENGLISH: "Blacklist entries must contain \"windowTitle\".",
            Language.GERMAN: "Blacklist-Einträge müssen \"windowTitle\" enthalten."
        ],
        AppString.ERROR_ERROR_OCCURRED_WHILE_STARTING: [
            Language.ENGLISH: "An error occurred while starting ReNeo:\n%s",
            Language.GERMAN: "Beim Starten von ReNeo ist ein Fehler aufgetreten:\n%s"
        ],
        AppString.ERROR_WHILE_INITIALIZING: [
            Language.ENGLISH: "Error during initialization",
            Language.GERMAN: "Fehler beim Initialisieren"
        ],
        AppString.ERROR_PATH_DOES_NOT_EXIST: [
            Language.ENGLISH: "%s does not exist.",
            Language.GERMAN: "%s existiert nicht."
        ],
        AppString.ERROR_WHILE_PARSING: [
            Language.ENGLISH: "Error while parsing %s.\n%s",
            Language.GERMAN: "Fehler beim Parsen von %s.\n%s"
        ],
    ];

    selectedLanguage = lang;
}

string appString(T...)(AppString as, T args) nothrow {
    try {
        if (as in stringMap && selectedLanguage in stringMap[as]) {
            return format(stringMap[as][selectedLanguage], args);
        } else {
            return as.to!string;
        }
    } catch (Exception e) {
        return "";
    }
}

LPCWSTR appStringwz(T...)(AppString as, T args) nothrow {
    try {
        return appString(as, args).toUTF16z;
    } catch (Exception e) {
        return null;
    }
}

string hotkeyString(HotkeyConfig hotkey) {
    string hotkeyString = "";

    if (hotkey.modFlags & MOD_WIN) {
        hotkeyString ~= "Win+";
    }
    if (hotkey.modFlags & MOD_CONTROL) {
        hotkeyString ~= controlKeyName() ~ "+";
    }
    if (hotkey.modFlags & MOD_ALT) {
        hotkeyString ~= "Alt+";
    }
    if (hotkey.modFlags & MOD_SHIFT) {
        hotkeyString ~= "Shift+";
    }

    hotkeyString ~= hotkey.key.to!VKEY.to!string[3..$].capitalize;

    return hotkeyString;
}

string controlKeyName() {
    if (selectedLanguage == Language.GERMAN) {
        return "Strg";
    } else {
        return "Ctrl";
    }
}
