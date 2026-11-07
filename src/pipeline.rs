//! One worker owns recording and inference; commands are serialized.
use crate::{audio, engines::Cache, inject, models, settings::SettingsStore};
use anyhow::{bail, Result};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Default, Clone)]
pub struct Status {
    pub phase: &'static str,
    pub notice: String,
    pub transcript: String,
    pub level: f32,
    pub history: Vec<crate::transcript::Entry>,
    pub transcription_ms: Option<u64>,
}
enum Command {
    Text(String),
    Shutdown(mpsc::Sender<()>),
}
#[derive(Clone)]
pub struct Controller {
    pub status: Arc<Mutex<Status>>,
    tx: mpsc::Sender<Command>,
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
            let mut model_cache = Cache::default();
            let mut recording_settings = None;
            let mut target = inject::Target::default();
            #[cfg(target_os = "macos")]
            let mut output_mute = None;
            let mut started = std::time::Instant::now();
            loop {
                let listening = status.lock().unwrap().phase == "listening";
                let timeout = if listening {
                    std::time::Duration::from_secs(120).saturating_sub(started.elapsed())
                } else {
                    let seconds = settings.lock().unwrap().usize("unload_seconds");
                    std::time::Duration::from_secs(if seconds == 0 {
                        31_536_000
                    } else {
                        seconds.min(1800) as u64
                    })
                };
                let command = match rx.recv_timeout(timeout) {
                    Ok(Command::Text(command)) => command,
                    Ok(Command::Shutdown(reply)) => {
                        recorder.cancel();
                        #[cfg(target_os = "macos")]
                        drop(output_mute.take());
                        let _ = reply.send(());
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) if listening => "stop".into(),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        model_cache = Cache::default();
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
                        "start" | "start_preview" if phase == "idle" => {
                            let mut config = settings.lock().unwrap().clone();
                            if command == "start_preview" {
                                config.set_quiet("injection", "none".into());
                            }
                            let model = models::find(&config.str("model"))?;
                            if !models::is_downloaded(model) {
                                bail!("Download {} in Models first", model.name);
                            }
                            target = inject::Target::capture();
                            recorder.start(config.opt_str("microphone"))?;
                            if config.bool("audio_feedback") {
                                audio::feedback(
                                    config.opt_str("output_device"),
                                    config.usize("feedback_volume") as f32 / 100.0,
                                );
                            }
                            #[cfg(target_os = "macos")]
                            if config.bool("mute_recording") {
                                match crate::system::OutputMute::new() {
                                    Ok(mute) => output_mute = Some(mute),
                                    Err(err) => log::warn!("{err}"),
                                }
                            }
                            recording_settings = Some(config);
                            started = std::time::Instant::now();
                            status.lock().unwrap().phase = "listening";
                            status.lock().unwrap().notice.clear();
                            ctx.request_repaint();
                            model_cache.prepare(model);
                        }
                        "stop" if phase == "listening" => {
                            let stopped = std::time::Instant::now();
                            status.lock().unwrap().phase = "working";
                            ctx.request_repaint();
                            #[cfg(target_os = "macos")]
                            {
                                output_mute = None;
                            }
                            let samples = recorder.stop()?;
                            let config = recording_settings.take().unwrap();
                            if samples.len() < audio::TARGET_RATE as usize / 10 {
                                bail!("Recording was too short");
                            }
                            let samples = if config.bool("vad") {
                                crate::transcript::speech_slice(&samples)
                            } else {
                                &samples
                            };
                            if samples.is_empty() {
                                bail!("No speech detected. Try moving closer to the microphone.");
                            }
                            if config.bool("audio_feedback") {
                                audio::feedback(
                                    config.opt_str("output_device"),
                                    config.usize("feedback_volume") as f32 / 100.0,
                                );
                            }
                            let id = config.str("model");
                            let engine = model_cache.get()?;
                            let inference = std::time::Instant::now();
                            let text = engine.transcribe(samples, &config)?;
                            let inference_ms = inference.elapsed().as_millis() as u64;
                            let text = crate::transcript::clean(&text, &config);
                            if text.trim().is_empty() {
                                bail!("No speech recognized");
                            }
                            // Preserve the result even if insertion fails.
                            {
                                let mut state = status.lock().unwrap();
                                state.transcript = text.clone();
                                state.transcription_ms = Some(inference_ms);
                                state.history.insert(
                                    0,
                                    crate::transcript::Entry {
                                        id: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_micros()
                                            as u64,
                                        text: text.clone(),
                                        model: id.clone(),
                                    },
                                );
                                crate::transcript::retain(
                                    &mut state.history,
                                    config.usize("history_limit"),
                                );
                            }
                            let delivery = inject::inject(&text, &config, &target);
                            log::info!("Dictation: model={id}, audio_ms={}, inference_ms={inference_ms}, stop_to_delivery_ms={}, delivered={}", samples.len() * 1000 / audio::TARGET_RATE as usize, stopped.elapsed().as_millis(), delivery.is_ok());
                            if delivery.is_err() {
                                // A hidden error is useless when dictating into another app.
                                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
                                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(
                                    false,
                                ));
                                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
                            }
                            delivery?;
                            status.lock().unwrap().phase = "idle";
                        }
                        "cancel" if phase == "listening" => {
                            #[cfg(target_os = "macos")]
                            {
                                output_mute = None;
                            }
                            recorder.cancel();
                            recording_settings = None;
                            status.lock().unwrap().phase = "idle";
                        }
                        command if phase == "idle" && command.starts_with("delete:") => {
                            let model = models::find(&command[7..])?;
                            model_cache = Cache::default();
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
            #[cfg(target_os = "macos")]
            drop(output_mute);
        });
        controller
    }
    pub fn send(&self, command: &str) {
        if self.status.lock().unwrap().phase != "working" {
            let _ = self.tx.send(Command::Text(command.into()));
        }
    }
    pub fn shutdown(&self) {
        let (tx, rx) = mpsc::channel();
        if self.tx.send(Command::Shutdown(tx)).is_ok() {
            // While transcribing, output is already restored. Never hold quit
            // indefinitely for a model task or an unavailable audio device.
            let _ = rx.recv_timeout(std::time::Duration::from_secs(2));
        }
    }
    pub fn notice(&self, text: impl Into<String>) {
        self.status.lock().unwrap().notice = text.into();
    }
}
