//! One worker owns recording and inference; commands are serialized.
use crate::{audio, engines::Loaded, inject, models, settings::SettingsStore};
use anyhow::{bail, Result};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Default, Clone)]
pub struct Status {
    pub phase: &'static str,
    pub notice: String,
    pub transcript: String,
    pub level: f32,
}
#[derive(Clone)]
pub struct Controller {
    pub status: Arc<Mutex<Status>>,
    tx: mpsc::Sender<String>,
    pub settings: Arc<Mutex<SettingsStore>>,
}
impl Controller {
    pub fn new(ctx: eframe::egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(Status {
            phase: "idle",
            ..Default::default()
        }));
        let settings = Arc::new(Mutex::new(SettingsStore::load()));
        let controller = Self {
            status: status.clone(),
            tx,
            settings: settings.clone(),
        };
        let recorder = audio::AudioRecorder::new(Arc::new({
            let status = status.clone();
            let ctx = ctx.clone();
            move |level| {
                status.lock().unwrap().level = level;
                ctx.request_repaint();
            }
        }));
        std::thread::spawn(move || {
            let mut loaded: Option<(String, Loaded)> = None;
            let mut recording_settings = None;
            let mut started = std::time::Instant::now();
            loop {
                let listening = status.lock().unwrap().phase == "listening";
                let timeout = if listening {
                    std::time::Duration::from_secs(120).saturating_sub(started.elapsed())
                } else {
                    std::time::Duration::from_secs(60)
                };
                let command = match rx.recv_timeout(timeout) {
                    Ok(command) => command,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) if listening => "stop".into(),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        loaded = None;
                        continue;
                    }
                };
                let phase = status.lock().unwrap().phase;
                let command = if command == "toggle" {
                    if phase == "listening" {
                        "stop"
                    } else {
                        "start"
                    }
                } else {
                    &command
                };
                let result: Result<()> = (|| {
                    match command {
                        "start" if phase == "idle" => {
                            let config = settings.lock().unwrap().clone();
                            let model = models::find(&config.str("model"))?;
                            if !models::is_downloaded(model) {
                                bail!("Download {} in Models first", model.name);
                            }
                            recorder.start(config.opt_str("microphone"))?;
                            recording_settings = Some(config);
                            started = std::time::Instant::now();
                            status.lock().unwrap().phase = "listening";
                        }
                        "stop" if phase == "listening" => {
                            status.lock().unwrap().phase = "working";
                            ctx.request_repaint();
                            let samples = recorder.stop()?;
                            let config = recording_settings.take().unwrap();
                            if samples.len() < audio::TARGET_RATE as usize / 10 {
                                bail!("Recording was too short");
                            }
                            let id = config.str("model");
                            if loaded.as_ref().is_none_or(|(current, _)| current != &id) {
                                // Release the previous engine before allocating another model.
                                loaded = None;
                                loaded = Some((id.clone(), Loaded::load(models::find(&id)?)?));
                            }
                            let text = loaded.as_mut().unwrap().1.transcribe(&samples, &config)?;
                            if text.trim().is_empty() {
                                bail!("No speech recognized");
                            }
                            // Preserve the result even if insertion fails.
                            status.lock().unwrap().transcript = text.clone();
                            inject::inject(&text, &config)?;
                            status.lock().unwrap().phase = "idle";
                        }
                        "cancel" if phase == "listening" => {
                            recorder.cancel();
                            recording_settings = None;
                            status.lock().unwrap().phase = "idle";
                        }
                        command if phase == "idle" && command.starts_with("delete:") => {
                            let model = models::find(&command[7..])?;
                            loaded = None;
                            models::delete(model)?;
                        }
                        _ => {}
                    }
                    Ok(())
                })();
                if let Err(err) = result {
                    let mut status = status.lock().unwrap();
                    status.phase = "idle";
                    status.notice = format!("{err:#}");
                }
                ctx.request_repaint();
            }
            recorder.cancel();
        });
        controller
    }
    pub fn send(&self, command: &str) {
        if self.status.lock().unwrap().phase != "working" {
            let _ = self.tx.send(command.into());
        }
    }
    pub fn notice(&self, text: impl Into<String>) {
        self.status.lock().unwrap().notice = text.into();
    }
}
