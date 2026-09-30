module logging;

// Debug output. Developer debug builds ("dub build") print everything to the console. The debug build that is
// shipped with releases (reneo_debug.exe, "dub build --build=debug-log") additionally writes a log file that
// users attach to bug reports, so it must be safe to share:
// - Anything that could reveal typed text (keys that produce text, characters, compose input, window titles)
//   is logged with debugWritelnPrivate and replaced by a placeholder in this build.
// - Logging only starts after the user agreed in a dialog, and old log files are deleted automatically.

import std.stdio : File, writeln;
import std.datetime.systime : Clock, SysTime;
import std.format : format;

void debugWriteln(T...)(T args) nothrow {
    debug {
        // Keep the test output readable
        version (unittest) {} else {
            try {
                writeln(args);
            } catch (Exception e) {}
        }
    }

    version (FileLogging) {
        writeToLogFile(args);
    }
}

// For log lines that may contain typed text. The redacted text replaces them in the shipped debug build.
void debugWritelnPrivate(T...)(string redacted, T args) nothrow {
    version (FileLogging) {
        debugWriteln(redacted);
    } else {
        debugWriteln(args);
    }
}

import std.algorithm : startsWith, endsWith;
import std.file : dirEntries, SpanMode, mkdirRecurse, remove, exists;
import std.path : buildPath, baseName;
import core.time : days;

enum LOG_FILE_PREFIX = "reneo_log_";

// Delete log files older than retentionDays from logDir
void deleteOldLogFiles(string logDir, int retentionDays, SysTime now) nothrow {
    try {
        if (!exists(logDir)) {
            return;
        }
        foreach (entry; dirEntries(logDir, SpanMode.shallow)) {
            string name = baseName(entry.name);
            if (entry.isFile && name.startsWith(LOG_FILE_PREFIX) && name.endsWith(".txt")
                    && now - entry.timeLastModified > retentionDays.days) {
                remove(entry.name);
            }
        }
    } catch (Exception e) {}
}

version (FileLogging) {
    // Stop writing to a single log file after this size, so it can't fill the disk
    enum MAX_LOG_FILE_SIZE = 20 * 1024 * 1024;

    private File logFile;
    private string logFilePath;

    string currentLogFilePath() nothrow {
        return logFilePath;
    }

    // Write buffered data to disk (used by tests)
    void flushLogFile() nothrow {
        try {
            if (logFile.isOpen) {
                logFile.flush();
            }
        } catch (Exception e) {}
    }

    void closeLogFile() nothrow {
        try {
            logFile.close();
        } catch (Exception e) {}
    }
    private bool logDecided;  // until the user decided, lines are buffered in memory
    private string[] pendingLines;
    private size_t logFileSize;

    bool fileLoggingActive() nothrow {
        return logFile.isOpen;
    }

    private void writeToLogFile(T...)(T args) nothrow {
        try {
            auto currTime = Clock.currTime();
            string line = format("%04d-%02d-%02d %02d:%02d:%02d.%03d ", currTime.year(), currTime.month(), currTime.day(),
                currTime.hour(), currTime.minute(), currTime.second(), cast(int) currTime.fracSecs().total!"msecs");
            foreach (arg; args) {
                line ~= format("%s", arg);
            }

            if (!logDecided) {
                pendingLines ~= line;
            } else if (logFile.isOpen && logFileSize < MAX_LOG_FILE_SIZE) {
                logFile.writeln(line);
                logFile.flush();  // flush immediately in case we crash
                logFileSize += line.length + 1;
            }
        } catch (Exception e) {}
    }

    // Start writing the log file (enabled == true) or discard everything that was buffered (enabled == false)
    void startFileLogging(bool enabled, string logDir, int retentionDays) nothrow {
        logDecided = true;

        try {
            deleteOldLogFiles(logDir, retentionDays, Clock.currTime());
        } catch (Exception e) {}

        if (enabled) {
            try {
                mkdirRecurse(logDir);
                auto now = Clock.currTime();
                string fileName = format("%s%04d-%02d-%02d_%02d%02d%02d.txt", LOG_FILE_PREFIX, now.year(), now.month(),
                    now.day(), now.hour(), now.minute(), now.second());
                logFilePath = buildPath(logDir, fileName);
                logFile = File(logFilePath, "w");
                foreach (line; pendingLines) {
                    logFile.writeln(line);
                    logFileSize += line.length + 1;
                }
                logFile.flush();
            } catch (Exception e) {}
        }

        pendingLines = [];
    }
}
