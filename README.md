# Bolum

A small, local voice dictation desktop app inspired by [Handy](https://github.com/cjpais/Handy).
Press a global shortcut, speak, press it again, and Bolum inserts the transcript into the
focused application. Hold-to-talk is also available.

The desktop interface, audio capture, shortcuts, model downloads, inference integration,
and text insertion are written in Rust. The interface uses egui with OpenGL. There is no
webview, JavaScript, Node, Python runtime, cloud API, or LLM cleanup service.
NVIDIA models run locally through ONNX Runtime's CPU provider.

## Build and use

Install stable Rust (1.92+) and your platform's native build tools, then:

```sh
cargo run --release
```

Open **Models**, download one model, select **Use model**, and focus a text field in
another app. The default shortcut is **Control + Alt + Space**. Enable hold-to-talk if
preferred. Downloads resume and each newly downloaded artifact is checked against its
pinned SHA-256. The first transcription loads the model; subsequent recordings reuse it.
Models unload after 60 seconds idle by default; change this in Advanced. Recordings stop
automatically at two minutes. Auto shortcut mode supports a tap to toggle or a hold to talk.

The Handy-inspired interface has General, History, Models, Advanced, and About sections,
with purple accents, grouped settings rows, an icon sidebar, and a persistent model footer.
Choose System, Light, or Dark appearance in About. On macOS it uses the installed San
Francisco font. In **General**, choose **Record a sample** to test your microphone and model
without inserting text into another app.

General includes shortcut capture, Auto / toggle / hold behavior, Canary language and
English translation, microphone selection, feedback tones and volume. macOS can temporarily
mute output during recording and restores the previous state on stop, cancel, or normal quit.
Advanced includes the recording overlay, model-unload timing, insertion mode, optional Enter
after insertion, energy-based quiet-edge trimming, English filler removal, exact custom-word
corrections, and the session-history limit. No extra cloud service is involved.

The window displays errors and the last transcript, which can be copied if insertion fails.
Transcripts and recordings are not written to disk. History keeps up to 20 entries in memory
by default (configurable up to 100, or Off); quitting clears it. Paste mode replaces the system clipboard.
Closing the window quits the app; **Hide to tray** keeps it running on macOS and Windows.
Linux currently uses the normal window/minimize controls rather than a system tray.
Command-comma opens General, Command-W hides the window when the tray is available,
Command-Q quits, and Escape cancels an active recording. On other platforms, the command
shortcuts use Control.

### macOS

Install Xcode Command Line Tools (`xcode-select --install`). Build a Finder-launchable app:

```sh
sh scripts/package-macos.sh
open target/bundle/Bolum.app
```

The script creates an ad-hoc signed `.app` and ZIP. For distribution to other machines,
Developer ID signing and notarization are still needed. The bundle declares microphone
usage. Allow Microphone and Accessibility access under System Settings → Privacy & Security.
When running via Cargo, macOS may associate permissions with the terminal instead.
The configured deployment target is macOS 13.4; compatibility with the oldest target still
needs testing on that OS.

### Windows

Use the MSVC Rust toolchain and Visual Studio C++ Build Tools. Build with
`cargo build --release`; the executable is `target/release/bolum.exe`. No WebView2 is needed.
The CI artifact also collects any runtime DLLs produced by the build.

### Linux

On Debian/Ubuntu install native build dependencies:

```sh
sudo apt install build-essential pkg-config libasound2-dev libx11-dev libxi-dev libxrandr-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev
```

Install `wtype` for Wayland or `xdotool` for X11 for typing. Paste/copy also needs
`wl-clipboard`, `xclip`, or `xsel`. Other supported typing tools are `dotool`, `ydotool`,
and `kwtype`. Global shortcut registration supports X11; on Wayland bind the CLI in your
compositor, for example in Hyprland:

```ini
exec-once = /absolute/path/to/bolum
bind = SUPER, D, exec, /absolute/path/to/bolum toggle
```

The ONNX prebuilt runtime may require a recent glibc; CI builds on Ubuntu 24.04.

## Models and application size

Weights download separately into `~/.local/share/bolum/models` (Windows:
`%LOCALAPPDATA%/bolum/models`). They are excluded from the app bundle. Existing NVIDIA
model downloads from the previous version can be reused.

- **Canary 180M Flash**, approximately 213 MB: English, German, Spanish, French. Select the
  spoken language explicitly. This is the smallest model offered.
- **Parakeet TDT 0.6B v3**, approximately 670 MB: automatic recognition across 25 European
  languages; the default selection.
- **Parakeet TDT 0.6B v2**, approximately 661 MB: English.

These are community int8 ONNX exports of NVIDIA models. Size is the total of the catalog
artifacts in decimal MB. CPU inference avoids shipping CUDA, PyTorch, or NeMo. Release
builds use size optimization, LTO, one codegen unit, and stripped symbols. ONNX Runtime
is still a substantial native dependency, so model size and app size must be reported
separately. Measure a build with `wc -c target/release/bolum` and `du -sh target/bundle/Bolum.app`.
The verified local Apple Silicon release executable is 25,710,368 bytes (about 26 MB);
the macOS app bundle is approximately the same size, excluding models. Other platforms
need their own measurements. Actual memory use during inference is larger than the model files.

Settings are stored in `~/.config/bolum/settings.json`, or `%APPDATA%/bolum/settings.json`.
`XDG_CONFIG_HOME` and `XDG_DATA_HOME` override these locations on all platforms. Old Whisper
selections migrate to Parakeet v3. Obsolete settings are discarded when saving; existing
history and unused weights are left on disk for the user to remove if desired.

## CLI and verification

```sh
bolum toggle                 # queue start/stop in the running desktop app
bolum start
bolum stop
bolum cancel                 # cancel a recording
bolum status                 # idle | listening | working
bolum show
bolum quit

cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

A real inference test downloads Canary and Parakeet v3 into an isolated directory, verifies
the artifacts, and checks that both recognize the speech fixture:

```sh
XDG_DATA_HOME=/tmp/bolum-smoke-data XDG_CONFIG_HOME=/tmp/bolum-smoke-config \
  cargo test --test smoke -- --ignored --nocapture
```

To transcribe an existing WAV with a downloaded model:

```sh
cargo run --release --example transcribe -- canary-180m-flash tests/fixtures/quick-brown-fox.wav en
```

CI checks formatting, lint, tests, and release compilation on macOS, Windows, and Linux.
It uploads build artifacts; it does not automatically publish releases. See [UI verification](docs/ui-verification.md) for interaction coverage, visual review,
live macOS checks, and remaining platform/permission checks.

## Architecture and research

`src/ui.rs` is the Rust window and tray; `pipeline.rs` serializes dictation on one worker;
`audio.rs` captures mono speech and resamples to 16 kHz; `engines/onnx.rs` runs the NVIDIA
graphs; `models.rs` contains the catalog and resumable downloader; `inject.rs` inserts
text; `ipc.rs` exposes a local socket/named pipe for compositor shortcuts.

The scope follows [Handy's local dictation workflow](https://github.com/cjpais/Handy),
with a Rust UI replacing its web frontend. The model choices follow NVIDIA's
[Canary 180M Flash model card](https://huggingface.co/nvidia/canary-180m-flash) and
[Parakeet v3 model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3).
Graph interfaces and decoding follow [onnx-asr](https://github.com/istupakov/onnx-asr),
using [istupakov's ONNX exports](https://huggingface.co/istupakov).
CPU selection is explicit: automatic CoreML selection fails on Canary's zero-length
initial decoder cache. NVIDIA model weights are licensed CC-BY-4.0; the application is
MIT licensed, see [LICENSE](LICENSE).
