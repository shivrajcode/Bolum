# Desktop UI verification

Verified locally on macOS / Apple Silicon, 2026-11-07.

## Interface

The production Rust/egui interface uses a light lavender sidebar and selection color,
neutral content surfaces, native window controls, and the installed San Francisco font
on macOS. Fonts are read from the OS rather than packaged with Bolum. The five sections
are General, Speech models, Audio input, Last dictation, and About & permissions.
The layout borrows the straightforward settings organization of
[Handy](https://github.com/cjpais/Handy). It is rendered in Rust, not a webview or an AppKit
control implementation.

## Automated interaction tests

`cargo test --locked --all-targets` passes 35 tests. Twelve exercise the actual production
view with egui_kittest using pointer events, keyboard events, and accessibility queries:

| Area | Verified behavior |
| --- | --- |
| Navigation | All five sections and the first-run model setup link |
| Shortcut | Invalid input stays inline; valid input emits registration intent |
| Preferences | Hold-to-talk, trailing space, and all three delivery modes save correctly |
| Models | Download/cancel, selection, Canary language, and two-step removal |
| Audio | Device selection, refresh, sample-recording intent, stop and cancel |
| Missing setup | Missing microphone and model show recovery controls |
| Busy state | Settings lock during transcription; navigation remains available |
| Transcript | Empty state, microphone-test link, copy and clear actions |
| Notices | Dismiss control emits the correct action |
| Window/permissions | Hide, quit, and permission buttons emit the correct actions |
| Keyboard | Tab focus, Command-comma, Command-W, Command-Q, Escape cancellation |
| Small windows | Recording buttons fit within the minimum 720 × 520 viewport |

These tests do not grant OS permissions, change the real clipboard, delete models, or
perform network downloads. Production action handlers route their intents to the existing
Rust services. Tests and rendering dependencies are development-only.

Strict Clippy, formatting, and whitespace checks also pass.

## Visual checks

The actual view was rendered and inspected at 820 × 660 and 720 × 520 for all five
sections, plus recording and transcription states. Language selection was moved into
the Canary row so it stays next to the selected model. Recording controls wrap at narrow
widths. Content scrolls vertically when it exceeds the available height.

To reproduce PNGs under `target/ui-review/`:

```sh
cargo test --lib ui::tests::render_all_pages_for_visual_review -- --ignored
```

These images render production widgets through the test GPU renderer; they are not
screenshots of macOS window chrome. OS window screenshot capture was unavailable in this
session. Pixel snapshots are kept out of the repository and installer.

## Packaged application checks

Using an isolated `XDG_CONFIG_HOME` and the installed Canary model:

- Launched the packaged `.app` successfully with the macOS system font.
- Opened all five sections through the native accessibility tree.
- Resized the real window to its 720 × 520 minimum.
- Refreshed microphone devices.
- Hid the window to the menu bar and restored it with `bolum show`.
- Started and finished a microphone sample through the real UI.

The sample path overrides insertion to `none` for that recording, so the test does not
paste into another app and does not change the user's saved delivery preference.

Separately, the opt-in inference integration test passed for Canary 180M Flash and
Parakeet v3. Both produced: “The quick brown fox jumps over the lazy dog.”

```sh
XDG_DATA_HOME=/tmp/bolum-smoke-data XDG_CONFIG_HOME=/tmp/bolum-smoke-config \
  cargo test --test smoke -- --ignored --nocapture
```

## Remaining platform checks

Cross-application typing and pasting remain unverified because macOS Accessibility
access was unavailable for text injection in this session. Permission buttons are wired,
but a human must grant permission in System Settings. Live shortcut interception and
permission-denial/recovery behavior still need testing across other app focus states.
Windows and Linux native windows were not exercised locally; CI runs the platform build,
lint, and headless test suite. macOS 13.4 compatibility requires a machine on that OS.
