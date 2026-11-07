//! Small OS helpers. No process is invoked until its corresponding control is used.
use anyhow::{anyhow, Result};
#[cfg(target_os = "macos")]
pub struct OutputMute {
    previous: bool,
}
#[cfg(target_os = "macos")]
impl OutputMute {
    pub fn new() -> Result<Self> {
        let output = std::process::Command::new("/usr/bin/osascript")
            .args(["-e", "output muted of (get volume settings)"])
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("Cannot read system mute state"));
        }
        let previous = String::from_utf8_lossy(&output.stdout).trim() == "true";
        let status = std::process::Command::new("/usr/bin/osascript")
            .args(["-e", "set volume output muted true"])
            .status()?;
        if !status.success() {
            return Err(anyhow!("Cannot mute system output"));
        }
        Ok(Self { previous })
    }
}
#[cfg(target_os = "macos")]
impl Drop for OutputMute {
    fn drop(&mut self) {
        let _ = std::process::Command::new("/usr/bin/osascript")
            .args([
                "-e",
                if self.previous {
                    "set volume output muted true"
                } else {
                    "set volume output muted false"
                },
            ])
            .status();
    }
}
pub fn open(target: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("/usr/bin/open");
    #[cfg(windows)]
    let mut command = {
        let mut c = std::process::Command::new("explorer.exe");
        c.arg(target);
        c
    };
    #[cfg(not(any(target_os = "macos", windows)))]
    let mut command = std::process::Command::new("xdg-open");
    #[cfg(not(windows))]
    command.arg(target);
    let status = command.status()?;
    if !status.success() {
        return Err(anyhow!("Could not open {target}"));
    }
    Ok(())
}
