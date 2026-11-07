# Desktop UI verification

Verified locally on macOS / Apple Silicon, 2026-11-07.

## Implemented interface

The Rust/egui layout follows the supplied Handy references: General, History, Models,
Advanced, About; an icon sidebar; grouped settings rows; switches; and a persistent model
footer. Purple replaces pink throughout light and dark appearance. System appearance is
also available. macOS uses its installed San Francisco fonts without bundling font files.
Phosphor supplies the small, consistent icon set. No webview or JavaScript was added.

General includes keyboard shortcut capture and reset, Auto/toggle/hold behavior, Canary
source language and English translation, microphone selection, feedback tones and volume,
and a sample-recording flow. macOS output muting restores its previous state on stop,
cancel, and normal application quit. Escape can cancel a recording globally on supported
desktops. The optional floating overlay does not focus the main settings window.

Models separates installed and downloadable weights, with a real search field, language
filter, active-model treatment, download progress, cancellation and removal confirmation.
Model speed/accuracy scores are not displayed because the app has no measured benchmarks.
Only the supported Parakeet and Canary models are listed.

Advanced controls map to working app behavior: start hidden when a tray exists, recording
overlay, model unload delay, insertion strategy, optional Enter after insertion,
energy-based quiet-edge detection, English filler removal, exact word corrections, and
bounded session history. The history is kept in memory and never writes recordings or
transcripts to disk. Startup registration, streaming, cloud cleanup, arbitrary clipboard
format restoration, and automatic updates are not implemented or presented as working
features.

## Automated checks

`cargo test --locked --all-targets`: **42 passed**, with the GPU review and real-download
integration tests opt-in. UI coverage includes eleven interaction workflows plus the runtime
shortcut mode transitions. The production view is tested using actual pointer/keyboard
events and accessibility queries, without OS or network side effects:

| Area | Verified behavior |
| --- | --- |
| Navigation | All sections, setup recovery, model footer |
| Shortcut | Capture, modifier validation, reset, Auto/toggle/hold command mapping |
| Sound settings | Microphone selection/refresh, feedback switch, output device |
| Recording | Sample intent, stop/cancel, busy-state locking and navigation |
| Models | Download/cancel, active selection, removal confirmation, Canary language/translation |
| Search/filter | Name matching, empty results, multilingual filter |
| Advanced | Unload timing, start hidden, overlay, insertion mode, cleanup, history limits |
| History | Empty state, latest transcript, copy/delete/clear action routing |
| Appearance/About | Light/dark/system, permissions, source and data-folder actions |
| Window/keyboard | Hide/quit shortcuts, Escape, notices and minimum-width recording controls |
| Paste permissions | Missing-permission action, banner removal after grant, show-only bypass |

Settings regression tests verify that a saved microphone choice survives reload and that
invalid null values do not erase typed defaults. Transcription helper tests verify exact
word boundaries, preservation of original whitespace when cleanup is disabled, silence
trimming, and history bounds.

Strict Clippy, formatting, and whitespace checks pass.

## Visual review

All five production screens were rendered in light and dark modes at 1000 × 780 and
760 × 560, generating twenty review images. Recording controls are also checked against
the minimum viewport. Large content sections scroll vertically.

```sh
cargo test --lib ui::tests::render_all_pages_for_visual_review -- --ignored
```

PNGs are under `target/ui-review/`, excluded from source control and the installer.
They render the actual production widgets through the test GPU renderer; they are not
screenshots of macOS chrome. OS window screenshot capture was unavailable in this session.

## Live macOS checks

Using an isolated configuration and the installed Canary model:

- Launched the packaged `.app` with system fonts.
- Opened General, History, Models, Advanced, and About through the native accessibility tree.
- Resized to the 760 × 560 minimum and refreshed microphone devices.
- Hid the settings window to the menu bar and restored it using `bolum show`.
- Started and finished a microphone sample through the real UI.
- Confirmed the floating recording window appeared.
- Confirmed system output muted during recording and returned to its original state on Stop.
- Quit during a second active recording and confirmed the original mute state was restored.
- With Finder focused, used Control-Option-Space to start/stop recording and global Escape
  to cancel; the control socket confirmed the expected listening/idle transitions.

The sample tests used insertion `none`, so no text was sent to other applications. The
application now explicitly asks its worker to finish recording cleanup on exit, with a
bounded wait to avoid hanging quit during model inference.

## Model inference

The opt-in integration test passes for Canary 180M Flash and Parakeet v3. Both recognize
“The quick brown fox jumps over the lazy dog.” from the fixture. It also verifies that
preloading, switching engines, and repeated dictations reuse the prepared model correctly.

```sh
XDG_DATA_HOME=/tmp/bolum-smoke-data XDG_CONFIG_HOME=/tmp/bolum-smoke-config \
  cargo test --test smoke -- --ignored --nocapture
```

## Limits

Cross-application typing/pasting still needs validation after granting the current Bolum
build Accessibility access. The running bundle reports `accessibility_ready: false`, even
after the first enable attempt and restart. The user deferred removing the stale entry and
adding the current ad-hoc-signed bundle. Successful paste and overlay focus restoration
must therefore not be described as verified. Feedback tone routing across different hardware,
non-English translation accuracy, and Windows/Linux live windows are not fully validated
here. The interface and decoder routing are tested; this does not establish recognition
accuracy for every language or microphone. macOS 13.4 compatibility needs that OS.
