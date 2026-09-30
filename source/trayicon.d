module trayicon;

import core.sys.windows.windows;

const UINT WM_TRAYICON = WM_USER + 10;

// Not (completely) in druntime's bindings
const UINT NOTIFYICON_VERSION_4 = 4;
const UINT NIF_SHOWTIP = 0x00000080;
const UINT NIN_SELECT = WM_USER + 0;
const UINT NIN_KEYSELECT = WM_USER + 1;
const UINT TPM_WORKAREA = 0x10000;

// NOTIFYICONDATAW as of Windows Vista (druntime only has the XP version without hBalloonIcon)
struct NotifyIconData {
    DWORD cbSize = NotifyIconData.sizeof;
    HWND hWnd;
    UINT uID;
    UINT uFlags;
    UINT uCallbackMessage;
    HICON hIcon;
    WCHAR[128] szTip = 0;
    DWORD dwState;
    DWORD dwStateMask;
    WCHAR[256] szInfo = 0;
    UINT uVersion;
    WCHAR[64] szInfoTitle = 0;
    DWORD dwInfoFlags;
    GUID guidItem;
    HICON hBalloonIcon;
}

extern (Windows) BOOL Shell_NotifyIconW(DWORD dwMessage, NotifyIconData* lpData) nothrow @nogc;

class TrayIcon {
    NotifyIconData nid;
    // Number of characters, the last one is reserved for the terminating null
    enum MAX_TIPLEN = nid.szTip.length - 1;

    bool visible = false;
    size_t tipLen;
    DWORD lastMenuClosed;

    this(HWND hwndParent, UINT id, HICON hicon, wchar[] tooltip) {
        nid.hWnd = hwndParent;
        nid.uID = id;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = hicon;
        // Version 4: the event is in the low word of lParam, the icon's anchor point in wParam, and
        // keyboard selection (Win+B, arrow keys, Enter) is reported as NIN_KEYSELECT
        nid.uVersion = NOTIFYICON_VERSION_4;

        this.setTip(tooltip);
    }

    ~this() {
        hide();
    }

    void show() nothrow {
        hide();
        Shell_NotifyIconW(NIM_ADD, &nid);
        Shell_NotifyIconW(NIM_SETVERSION, &nid);
        visible = true;
    }

    void hide() nothrow {
        if (visible) {
            Shell_NotifyIconW(NIM_DELETE, &nid);
            visible = false;
        }
    }

    void setTip(const(wchar)[] newTip) nothrow {
        tipLen = (newTip.length > MAX_TIPLEN) ? MAX_TIPLEN : newTip.length;
        nid.szTip[0 .. tipLen] = newTip[0 .. tipLen];
        nid.szTip[tipLen] = 0;
        if (visible) {
            Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    void setIcon(HICON hnewIcon) nothrow {
        nid.hIcon = hnewIcon;
        if (visible) {
            Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    // Shows the menu at the given point (the icon's anchor for keyboard selection, else the cursor).
    // Returns the ID of the selected menu item, or 0 if the menu was dismissed.
    UINT showContextMenu(HWND hwndParent, HMENU menu, POINT anchor) nothrow {
        // Required so that the menu closes when clicking elsewhere
        SetForegroundWindow(hwndParent);

        UINT flags = TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD | TPM_NONOTIFY | TPM_WORKAREA;
        flags |= GetSystemMetrics(SM_MENUDROPALIGNMENT) ? TPM_RIGHTALIGN : TPM_LEFTALIGN;
        UINT command = cast(UINT) TrackPopupMenuEx(menu, flags, anchor.x, anchor.y, hwndParent, NULL);

        // Also required for notification icon menus, otherwise the menu doesn't reliably open the next time
        // (see the remarks of TrackPopupMenu)
        PostMessage(hwndParent, WM_NULL, 0, 0);
        lastMenuClosed = GetTickCount();
        return command;
    }

    // Pressing space on a selected tray icon sends NIN_KEYSELECT twice. Ignore the second one, which
    // arrives right after the menu of the first one was closed.
    bool menuJustClosed() nothrow {
        return GetTickCount() - lastMenuClosed < 300;
    }
}
