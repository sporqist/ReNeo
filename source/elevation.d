module elevation;

// When ReNeo runs with administrator rights (e.g. via Task Scheduler "Run with highest privileges"), its program
// files must not be modifiable without administrator rights. Otherwise any program running as the same user
// could replace reneo.exe, plant a DLL next to it or change layouts.json, and would run with (or type into)
// administrator windows at the next logon.

import core.sys.windows.windows;
import core.sys.windows.aclapi : GetNamedSecurityInfoW;
import core.sys.windows.accctrl : SE_OBJECT_TYPE;

import std.file : exists, isDir;
import std.path : buildPath;
import std.utf : toUTF16z;

const DWORD LABEL_SECURITY_INFORMATION = 0x00000010;

bool isProcessElevated() nothrow {
    HANDLE token;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) {
        return false;
    }
    scope (exit) CloseHandle(token);

    TOKEN_ELEVATION elevation;
    DWORD size;
    if (!GetTokenInformation(token, TOKEN_INFORMATION_CLASS.TokenElevation, &elevation, elevation.sizeof, &size)) {
        return false;
    }
    return elevation.TokenIsElevated != 0;
}

// The non-elevated token of the same user, which every normal program of this user runs with. Returns null if
// there is none (e.g. UAC disabled or the built-in administrator account). Must be closed with CloseHandle.
HANDLE getLinkedToken() nothrow {
    HANDLE token;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) {
        return null;
    }
    scope (exit) CloseHandle(token);

    HANDLE linkedToken;
    DWORD size;
    if (!GetTokenInformation(token, TOKEN_INFORMATION_CLASS.TokenLinkedToken, &linkedToken, linkedToken.sizeof, &size)) {
        return null;
    }
    return linkedToken;
}

// Rights that allow replacing a file or planting new files in a directory
const DWORD[] FILE_MODIFY_RIGHTS = [FILE_WRITE_DATA, FILE_APPEND_DATA, DELETE, WRITE_DAC, WRITE_OWNER];
const DWORD[] DIRECTORY_MODIFY_RIGHTS = [FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_DELETE_CHILD, DELETE, WRITE_DAC, WRITE_OWNER];

// Could a process with the given impersonation token modify the file or directory at path?
bool canModify(HANDLE token, string path) nothrow {
    PSECURITY_DESCRIPTOR securityDescriptor;
    try {
        if (GetNamedSecurityInfoW(cast(wchar*) path.toUTF16z, SE_OBJECT_TYPE.SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION | LABEL_SECURITY_INFORMATION,
                null, null, null, null, &securityDescriptor) != ERROR_SUCCESS) {
            return false;
        }
    } catch (Exception e) {
        return false;
    }
    scope (exit) LocalFree(securityDescriptor);

    bool directory;
    try {
        directory = isDir(path);
    } catch (Exception e) {
        return false;
    }

    GENERIC_MAPPING mapping = GENERIC_MAPPING(FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_GENERIC_EXECUTE, FILE_ALL_ACCESS);

    // AccessCheck only succeeds if *all* requested rights are granted, so check each right on its own
    foreach (right; directory ? DIRECTORY_MODIFY_RIGHTS : FILE_MODIFY_RIGHTS) {
        DWORD desiredAccess = right;
        MapGenericMask(&desiredAccess, &mapping);

        PRIVILEGE_SET privileges;
        DWORD privilegesLength = privileges.sizeof;
        DWORD grantedAccess;
        BOOL accessStatus;
        if (AccessCheck(securityDescriptor, token, desiredAccess, &mapping, &privileges, &privilegesLength,
                &grantedAccess, &accessStatus) && accessStatus) {
            return true;
        }
    }

    return false;
}

// Returns the first program file or directory that can be modified without administrator rights, or null if
// everything is protected (or ReNeo doesn't run elevated, in which case there is nothing to gain).
string findUnprotectedProgramFile(string exeDir) nothrow {
    if (!isProcessElevated()) {
        return null;
    }

    HANDLE linkedToken = getLinkedToken();
    if (!linkedToken) {
        return null;
    }
    scope (exit) CloseHandle(linkedToken);

    string[] paths = [exeDir];
    try {
        foreach (name; ["reneo.exe", "cairo.dll", "layouts.json", "config.default.json", "keysymdef.h", "compose"]) {
            string path = buildPath(exeDir, name);
            if (exists(path)) {
                paths ~= path;
            }
        }
    } catch (Exception e) {}

    foreach (path; paths) {
        if (canModify(linkedToken, path)) {
            return path;
        }
    }

    return null;
}

unittest {
    import std.file : tempDir, write, remove;

    // Check the AccessCheck plumbing with the token of the test process itself
    HANDLE processToken;
    assert(OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY | TOKEN_DUPLICATE, &processToken));
    scope (exit) CloseHandle(processToken);
    HANDLE token;
    assert(DuplicateToken(processToken, SECURITY_IMPERSONATION_LEVEL.SecurityIdentification, &token));
    scope (exit) CloseHandle(token);

    string ownFile = buildPath(tempDir, "reneo_elevation_test.txt");
    write(ownFile, "");
    scope (exit) remove(ownFile);
    assert(canModify(token, ownFile));
    assert(canModify(token, tempDir));

    // A normal user can't modify system files. (If the tests run elevated, administrators still can't
    // modify files owned by TrustedInstaller.)
    string systemFile = buildPath(GetSystemDirectory(), "kernel32.dll");
    assert(!canModify(token, systemFile));
}

private string GetSystemDirectory() {
    import std.conv : to;
    wchar[MAX_PATH] buffer;
    uint length = GetSystemDirectoryW(buffer.ptr, MAX_PATH);
    return buffer[0 .. length].to!string;
}
