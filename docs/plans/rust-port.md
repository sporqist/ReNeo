# Rust port

**Status: in-flight** (started 2026-09-30)

ReNeo is ported from D to Rust. The D version (`source/`, branch `modernize`) stays the daily driver and
only gets bug fixes until the Rust version passes the parity gate (port/08). New UI work happens only in Rust.

Both versions read the same data files: `layouts.json`, `compose/*.module` / `*.remove`, `keysymdef.h`,
`config.json`. The D implementation and its tests (`source/tests.d`) are the behavioural reference: when the
two disagree and the plan doesn't say otherwise, the D behaviour wins.

## Crates

- `crates/reneo-core`: everything that can be tested without Windows. Keysyms, layouts, compose, and the key
  handling engine as a state machine: physical key events in, output actions out. Platform queries
  (native layout lookups, Capslock state) go through the `NativeLayout` trait so tests can fake them.
- `crates/reneo-win`: the Windows side. Keyboard hook thread, input injection, layout detection, tray, OSK,
  config, logging. Uses `windows` 0.62.

## Invariants

- **The hook path doesn't allocate, block or panic.** Windows silently removes low-level hooks that exceed
  `LowLevelHooksTimeout`. The engine works on pre-sized buffers; anything slow (OSK redraw, tray updates,
  config writes) is posted to the UI thread.
- **Injected input passes untouched.** Like the D version, the hook ignores all injected events: ReNeo's own, and
  those of other programs (e.g. a password manager's auto-type), which must not be remapped. ReNeo's own events
  additionally carry a marker in `dwExtraInfo`, so logs can tell them apart.
- **Logs never contain typed text.** Same rule as the D version's `debugWritelnPrivate`, see the D
  `source/logging.d`. Tested like `dub test --build=unittest-log`.
- **License: GPL-3.0**, like the D version. Every dependency must be GPL-3.0 compatible.

## Streams

| ID | Scope | Status |
|---|---|---|
| port/01 | Workspace, this plan, CI job | landed |
| port/02 | Keysyms, layouts (loading and validation) | landed |
| port/03 | Compose: parser, tree, special modes | landed |
| port/04 | Engine: layers, modifiers, locks, Capslock, numpad fake shift, one-handed mode, forced modifiers | landed |
| port/05 | reneo-win: hook thread, injector, native layout tables, layout detection, watchdog, session recovery | not started |
| port/06 | Config (incl. `%APPDATA%` fallback), redacted logging, elevation check, DLL hardening, localization | not started |
| port/07 | Tray: icon A states, native menu, tooltip | not started |
| port/08 | Parity gate: all ported tests pass, one to two weeks of daily use, then Rust replaces D | not started |
| ui/01+ | Direct2D OSK (Fluent, finger zones), flyout, settings window, dark menus | not started |

Designs: [ReNeo UI designs](https://claude.ai/artifact/Nj2wCtn5fpKUXZLveUTv4g) (approved: icon A, menu flyout C,
OSK Fluent light/dark with finger zones and numpad toggle, settings window).
