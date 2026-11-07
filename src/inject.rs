//! Text delivery into the focused application.
//!
//! Mirrors Handy's strategy: typing tools are detected on PATH (`wtype`,
//! `kwtype`, `dotool`, `ydotool`, `xdotool`) and the transcript is either typed
//! directly or written to the clipboard first and then pasted. On macOS and
//! Windows the built-in `native` tool sends key events instead (CGEvent, which
//! needs the Accessibility permission, and SendInput).

use anyhow::{anyhow, Result};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::settings::SettingsStore;

/// Capture before the recording overlay can take focus. Never redirect a
/// transcript to a different application if the user switches away mid-recording.
#[derive(Default)]
pub struct Target {
    #[cfg(target_os = "macos")]
    pid: Option<i32>,
}
impl Target {
    pub fn capture() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            pid: frontmost_pid().filter(|pid| *pid != std::process::id() as i32),
        }
    }
    fn restore(&self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
            let pid = self.pid.ok_or_else(|| {
                anyhow!("Place the cursor in another app and start dictation with your shortcut")
            })?;
            let current = frontmost_pid();
            if current == Some(pid) {
                return Ok(());
            }
            if current != Some(std::process::id() as i32) {
                return Err(anyhow!("The focused app changed during dictation. Paste with Command-V where you want the text"));
            }
            let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
                .ok_or_else(|| anyhow!("The dictation app was closed"))?;
            // Restore only when Bolum itself stole focus (e.g. the Finish button).
            if app.activateWithOptions(NSApplicationActivationOptions::empty()) {
                for _ in 0..30 {
                    if frontmost_pid() == Some(pid) {
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            Err(anyhow!(
                "Could not focus the dictation app. Paste with Command-V"
            ))
        }
        #[cfg(not(target_os = "macos"))]
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn frontmost_pid() -> Option<i32> {
    objc2_app_kit::NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

/// A read-only check: only the explicit permission action opens Apple's prompt.
pub fn accessibility_ready() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        unsafe extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        // No arguments or borrowed pointers; this only queries our own process.
        unsafe { AXIsProcessTrusted() }
    }
    #[cfg(not(target_os = "macos"))]
    true
}

#[cfg(not(any(target_os = "macos", windows)))]
const TOOLS: [&str; 5] = ["wtype", "kwtype", "dotool", "ydotool", "xdotool"];
#[cfg(any(target_os = "macos", windows))]
const TOOLS: [&str; 1] = ["native"];

pub fn which(program: &str) -> Option<std::path::PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

pub fn available_tools() -> Vec<&'static str> {
    TOOLS
        .into_iter()
        .filter(|t| *t == "native" || which(t).is_some())
        .collect()
}

fn resolve_tool(settings: &SettingsStore) -> Result<&'static str> {
    let requested = settings.str("typing_tool");
    if requested != "auto" {
        let tool = TOOLS
            .into_iter()
            .find(|t| *t == requested.as_str())
            .ok_or_else(|| anyhow!("unknown typing tool '{requested}'"))?;
        return Ok(tool);
    }
    available_tools()
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no typing tool found (install wtype, dotool or xdotool)"))
}

fn run_with_stdin(program: &str, args: &[&str], stdin: &str) -> Result<()> {
    let binary = which(program).ok_or_else(|| anyhow!("{program} not found"))?;
    let mut child = Command::new(binary)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(pipe) = child.stdin.as_mut() {
        pipe.write_all(stdin.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("{program} exited with {status}"));
    }
    Ok(())
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    let binary = which(program).ok_or_else(|| anyhow!("{program} not found"))?;
    let status = Command::new(binary)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(anyhow!("{program} exited with {status}"));
    }
    Ok(())
}

fn type_text(tool: &str, text: &str) -> Result<()> {
    match tool {
        "wtype" => run_with_stdin("wtype", &["-"], text),
        "kwtype" => run_with_stdin("kwtype", &["-"], text),
        "ydotool" => run("ydotool", &["type", "--", text]),
        // dotool treats a newline in its command stream as Enter.
        "dotool" => run_with_stdin(
            "dotool",
            &[],
            &format!("type {}\n", text.replace('\n', " ")),
        ),
        "xdotool" => run(
            "xdotool",
            &["type", "--clearmodifiers", "--delay", "2", "--", text],
        ),
        #[cfg(any(target_os = "macos", windows))]
        "native" => native::type_text(text),
        other => Err(anyhow!("cannot type with '{other}'")),
    }
}

fn write_clipboard(text: &str) -> Result<()> {
    #[cfg(any(target_os = "macos", windows))]
    return native::write_clipboard(text);
    #[allow(unreachable_code)]
    if which("wl-copy").is_some() && std::env::var("WAYLAND_DISPLAY").is_ok() {
        return run_with_stdin("wl-copy", &[], text);
    }
    if which("xclip").is_some() {
        return run_with_stdin("xclip", &["-selection", "clipboard"], text);
    }
    if which("xsel").is_some() {
        return run_with_stdin("xsel", &["--clipboard", "--input"], text);
    }
    Err(anyhow!("no clipboard tool found (wl-copy, xclip or xsel)"))
}

fn send_key(tool: &str, key: &str) -> Result<()> {
    match (tool, key) {
        ("wtype", "paste") => run("wtype", &["-M", "ctrl", "-P", "v", "-p", "v", "-m", "ctrl"]),
        ("wtype", "enter") => run("wtype", &["-P", "Return", "-p", "Return"]),
        ("kwtype", "paste") => run(
            "kwtype",
            &["-M", "ctrl", "-P", "v", "-p", "v", "-m", "ctrl"],
        ),
        ("kwtype", "enter") => run("kwtype", &["-P", "Return", "-p", "Return"]),
        ("xdotool", "paste") => run("xdotool", &["key", "--clearmodifiers", "ctrl+v"]),
        ("xdotool", "enter") => run("xdotool", &["key", "--clearmodifiers", "Return"]),
        ("ydotool", "paste") => run("ydotool", &["key", "29:1", "47:1", "47:0", "29:0"]),
        ("ydotool", "enter") => run("ydotool", &["key", "28:1", "28:0"]),
        ("dotool", "paste") => run_with_stdin("dotool", &[], "key ctrl+v\n"),
        ("dotool", "enter") => run_with_stdin("dotool", &[], "key enter\n"),
        #[cfg(any(target_os = "macos", windows))]
        ("native", key) => native::send_key(key),
        (tool, key) => Err(anyhow!("{tool} cannot send {key}")),
    }
}

/// Copies `text` to the system clipboard (tray "copy last transcript", history).
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    write_clipboard(text)
}

/// Sends `text` to the focused window using the configured strategy.
pub fn inject(text: &str, settings: &SettingsStore, target: &Target) -> Result<()> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let mut payload = text.to_string();
    if settings.bool("append_trailing_space") {
        payload.push(' ');
    }
    let mode = settings.str("injection");
    if mode == "none" {
        return Ok(());
    }
    let result = (|| {
        let tool = resolve_tool(settings)?;
        if mode == "clipboard" {
            write_clipboard(&payload)?;
        }
        if !accessibility_ready() {
            return Err(anyhow!("Enable Bolum in System Settings → Privacy & Security → Accessibility, then try the shortcut again"));
        }
        target.restore()?;
        match mode.as_str() {
            "clipboard" => {
                send_key(tool, "paste")?;
                if settings.bool("auto_submit") {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    send_key(tool, "enter")?;
                }
                Ok(())
            }
            "type" => {
                type_text(tool, &payload)?;
                if settings.bool("auto_submit") {
                    send_key(tool, "enter")?;
                }
                Ok(())
            }
            _ => Err(anyhow!("Unknown insertion method: {mode}")),
        }
    })();
    if let Err(err) = result {
        // Keep the result usable even when macOS blocks synthetic input.
        return match write_clipboard(&payload) {
            Ok(()) => Err(anyhow!("Transcript copied, but not pasted. {err:#}")),
            Err(copy_err) => Err(anyhow!(
                "Text is available in History. {err:#}. Clipboard: {copy_err}"
            )),
        };
    }
    Ok(())
}

/// Prompts for the Accessibility permission up front so the first dictation
/// is not the moment macOS asks for it.
#[cfg(target_os = "macos")]
pub fn request_permissions() -> Result<()> {
    #[cfg(target_os = "macos")]
    native::request_permissions()?;
    Ok(())
}

#[cfg(any(target_os = "macos", windows))]
mod native {
    use anyhow::{anyhow, Result};
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    /// The modifier that pastes: Command on macOS, Control on Windows.
    #[cfg(target_os = "macos")]
    const PASTE_MODIFIER: Key = Key::Meta;
    #[cfg(windows)]
    const PASTE_MODIFIER: Key = Key::Control;

    fn connect() -> Result<Enigo> {
        Enigo::new(&Settings {
            open_prompt_to_get_permissions: false,
            ..Settings::default()
        })
        .map_err(|err| {
            if cfg!(target_os = "macos") {
                anyhow!(
                    "{err}; allow Bolum under System Settings → Privacy & Security → Accessibility"
                )
            } else {
                anyhow!("{err}")
            }
        })
    }

    #[cfg(target_os = "macos")]
    pub fn request_permissions() -> Result<()> {
        Enigo::new(&Settings::default())
            .map(drop)
            .map_err(Into::into)
    }

    pub fn type_text(text: &str) -> Result<()> {
        connect()?
            .text(text)
            .map_err(|err| anyhow!("typing failed: {err}"))
    }

    pub fn send_key(key: &str) -> Result<()> {
        let mut enigo = connect()?;
        match key {
            "paste" => {
                enigo.key(PASTE_MODIFIER, Direction::Press)?;
                // Always release the modifier, even when the V press fails.
                let pressed = enigo.key(Key::Unicode('v'), Direction::Click);
                enigo.key(PASTE_MODIFIER, Direction::Release)?;
                pressed?;
            }
            "enter" => enigo.key(Key::Return, Direction::Click)?,
            other => return Err(anyhow!("cannot send {other}")),
        }
        Ok(())
    }

    /// `pbcopy` decodes stdin with the locale, which is unset for apps
    /// launched from Finder, so pin UTF-8 or accents arrive garbled.
    #[cfg(target_os = "macos")]
    pub fn write_clipboard(text: &str) -> Result<()> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut child = Command::new("/usr/bin/pbcopy")
            .env("LANG", "en_US.UTF-8")
            .stdin(Stdio::piped())
            .spawn()?;
        if let Some(pipe) = child.stdin.as_mut() {
            pipe.write_all(text.as_bytes())?;
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(anyhow!("pbcopy exited with {status}"));
        }
        Ok(())
    }

    #[cfg(windows)]
    pub fn write_clipboard(text: &str) -> Result<()> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text))
            .map_err(|err| anyhow!("clipboard: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_and_empty_results_do_not_need_permissions_target_or_typing_tool() {
        let mut settings = SettingsStore::load_defaults();
        settings.set_quiet("typing_tool", "missing-tool".into());
        settings.set_quiet("injection", "none".into());
        settings.set_quiet("auto_submit", true.into());
        assert!(inject("Sample stays local", &settings, &Target::default()).is_ok());
        settings.set_quiet("injection", "clipboard".into());
        assert!(inject(" \n", &settings, &Target::default()).is_ok());
    }
}
