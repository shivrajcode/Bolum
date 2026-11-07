//! Native window and OS services. Rendering is independently interaction-tested.
#[cfg(test)]
mod tests;
mod view;
use crate::{audio, inject, models, pipeline::Controller};
use anyhow::Result;
use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};
use view::{Action, ModelView, Screen, View};

pub fn run() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 780.0])
            .with_min_inner_size([760.0, 560.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Bolum",
        options,
        Box::new(|cc| Ok(Box::new(Desktop::new(cc)?))),
    )
    .map_err(|err| anyhow::anyhow!("desktop: {err}"))
}
struct Desktop {
    controller: Controller,
    runtime: tokio::runtime::Runtime,
    downloads: Arc<models::Downloads>,
    progress: Arc<Mutex<HashMap<String, f32>>>,
    microphones: Vec<String>,
    outputs: Vec<String>,
    cancel_hotkey: Option<HotKey>,
    shortcut_down: Option<std::time::Instant>,
    hotkeys: Option<GlobalHotKeyManager>,
    hotkey: Option<HotKey>,
    view: View,
    events: mpsc::Receiver<GlobalHotKeyEvent>,
    last_transcript: String,
    accessibility_ready: bool,
    permission_checked: std::time::Instant,
    #[cfg(any(target_os = "macos", windows))]
    tray: Option<tray_icon::TrayIcon>,
    #[cfg(any(target_os = "macos", windows))]
    tray_events: mpsc::Receiver<String>,
}
impl Desktop {
    fn new(cc: &eframe::CreationContext<'_>) -> Result<Self> {
        view::configure(&cc.egui_ctx, true);
        let ctx = cc.egui_ctx.clone();
        let controller = Controller::new(ctx.clone());
        let runtime = tokio::runtime::Runtime::new()?;
        crate::ipc::spawn_server(controller.clone(), ctx.clone(), &runtime);
        let (tx, events) = mpsc::channel();
        GlobalHotKeyEvent::set_event_handler(Some(move |event| {
            let _ = tx.send(event);
            ctx.request_repaint();
        }));
        let hotkeys = GlobalHotKeyManager::new()
            .map_err(|err| {
                controller.notice(format!("Shortcut unavailable: {err}. Use bolum toggle."))
            })
            .ok();
        let hotkey_text = controller.settings.lock().unwrap().str("hotkey");
        #[cfg(any(target_os = "macos", windows))]
        let (tray, tray_events) = create_tray(cc.egui_ctx.clone());
        let mut app = Self {
            controller,
            runtime,
            downloads: Arc::default(),
            progress: Arc::default(),
            microphones: audio::input_devices(),
            outputs: audio::output_devices(),
            cancel_hotkey: None,
            shortcut_down: None,
            hotkeys,
            hotkey: None,
            view: View {
                hotkey_text,
                ..Default::default()
            },
            events,
            last_transcript: String::new(),
            accessibility_ready: inject::accessibility_ready(),
            permission_checked: std::time::Instant::now(),
            #[cfg(any(target_os = "macos", windows))]
            tray,
            #[cfg(any(target_os = "macos", windows))]
            tray_events,
        };
        app.apply_hotkey();
        #[cfg(any(target_os = "macos", windows))]
        if app.tray.is_some() && app.controller.settings.lock().unwrap().bool("start_hidden") {
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        Ok(app)
    }
    fn apply_hotkey(&mut self) {
        let result = (|| -> Result<()> {
            let manager = self
                .hotkeys
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Global shortcuts unavailable; use bolum toggle"))?;
            let next: HotKey = self.view.hotkey_text.parse()?;
            if self.hotkey == Some(next) {
                return Ok(());
            }
            manager.register(next)?;
            let mut settings = self.controller.settings.lock().unwrap();
            let mut candidate = settings.clone();
            candidate.set_quiet("hotkey", self.view.hotkey_text.clone().into());
            if let Err(err) = candidate.save() {
                let _ = manager.unregister(next);
                return Err(err);
            }
            if let Some(old) = self.hotkey.replace(next) {
                let _ = manager.unregister(old);
            }
            *settings = candidate;
            Ok(())
        })();
        self.view.shortcut_error = result.err().map(|err| err.to_string()).unwrap_or_default();
    }
    fn download(&self, id: &'static str, ctx: egui::Context) {
        let Some(cancel) = self.downloads.start(id) else {
            return;
        };
        let model = models::find(id).unwrap();
        let downloads = self.downloads.clone();
        let progress = self.progress.clone();
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let result = models::download(model, cancel, |received, total| {
                progress
                    .lock()
                    .unwrap()
                    .insert(model.id.into(), received as f32 / total.max(1) as f32);
                ctx.request_repaint();
            })
            .await;
            downloads.finish(model.id);
            if let Err(err) = result {
                controller.notice(format!("Download: {err:#}"));
            }
            ctx.request_repaint();
        });
    }
    fn action(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::Save(settings) => match settings.save() {
                Ok(()) => {
                    crate::transcript::retain(
                        &mut self.controller.status.lock().unwrap().history,
                        settings.usize("history_limit"),
                    );
                    *self.controller.settings.lock().unwrap() = settings;
                    self.controller.send("config_changed");
                }
                Err(err) => self
                    .controller
                    .notice(format!("Could not save settings: {err}")),
            },
            Action::ApplyShortcut(value) => {
                self.view.hotkey_text = value;
                self.apply_hotkey();
            }
            Action::Command(command) => self.controller.send(command),
            Action::Download(id) => self.download(id, ctx.clone()),
            Action::CancelDownload(id) => self.downloads.cancel(id),
            Action::Delete(id) => self.controller.send(&format!("delete:{id}")),
            Action::RefreshMicrophones => {
                self.microphones = audio::input_devices();
                self.outputs = audio::output_devices();
            }
            Action::CopyEntry(text) => {
                if let Err(err) = inject::copy_to_clipboard(&text) {
                    self.controller.notice(format!("Could not copy: {err}"));
                }
            }
            Action::DeleteEntry(id) => {
                let mut status = self.controller.status.lock().unwrap();
                if status.history.first().is_some_and(|entry| entry.id == id) {
                    status.transcript.clear();
                }
                status.history.retain(|entry| entry.id != id);
            }
            Action::ClearHistory => {
                let mut status = self.controller.status.lock().unwrap();
                status.history.clear();
                status.transcript.clear();
            }
            Action::OpenData => {
                let path = crate::settings::data_dir();
                if let Err(err) = std::fs::create_dir_all(&path)
                    .map_err(anyhow::Error::from)
                    .and_then(|_| crate::system::open(&path.to_string_lossy()))
                {
                    self.controller.notice(err.to_string());
                }
            }
            Action::OpenSource => {
                if let Err(err) = crate::system::open("https://github.com/shivrajcode/Bolum") {
                    self.controller.notice(err.to_string());
                }
            }
            Action::Copy => {
                let text = self.controller.status.lock().unwrap().transcript.clone();
                match inject::copy_to_clipboard(&text) {
                    Ok(()) => self.view.copied = true,
                    Err(err) => self.controller.notice(format!("Could not copy: {err}")),
                }
            }
            Action::ClearTranscript => self.controller.status.lock().unwrap().transcript.clear(),
            Action::Dismiss => self.controller.notice(""),
            Action::Hide => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false)),
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            #[cfg(target_os = "macos")]
            Action::Permissions(kind) => {
                #[cfg(target_os = "macos")]
                {
                    let pane = if kind == "microphone" {
                        "Privacy_Microphone"
                    } else {
                        "Privacy_Accessibility"
                    };
                    if kind == "accessibility" {
                        let _ = inject::request_permissions();
                    }
                    if let Err(err) = std::process::Command::new("/usr/bin/open")
                        .arg(format!(
                            "x-apple.systempreferences:com.apple.preference.security?{pane}"
                        ))
                        .spawn()
                    {
                        self.controller
                            .notice(format!("Could not open System Settings: {err}"));
                    }
                }
                #[cfg(not(target_os = "macos"))]
                let _ = kind;
            }
        }
    }
}
fn shortcut_command(mode: &str, state: HotKeyState, held: bool) -> Option<&'static str> {
    match state {
        HotKeyState::Pressed => Some(if mode == "hold" { "start" } else { "toggle" }),
        HotKeyState::Released if mode == "hold" || mode == "auto" && held => Some("stop"),
        _ => None,
    }
}
impl eframe::App for Desktop {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.controller.shutdown();
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.permission_checked.elapsed() >= std::time::Duration::from_secs(1) {
            self.accessibility_ready = inject::accessibility_ready();
            self.permission_checked = std::time::Instant::now();
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        while let Ok(event) = self.events.try_recv() {
            if self.cancel_hotkey.is_some_and(|key| key.id() == event.id) {
                if event.state == HotKeyState::Pressed {
                    self.controller.send("cancel");
                }
                continue;
            }
            if self.hotkey.is_none_or(|key| key.id() != event.id) {
                continue;
            }
            let mode = self
                .controller
                .settings
                .lock()
                .unwrap()
                .str("shortcut_mode");
            let held = if event.state == HotKeyState::Pressed {
                self.shortcut_down = Some(std::time::Instant::now());
                false
            } else {
                self.shortcut_down
                    .take()
                    .is_some_and(|time| time.elapsed() >= std::time::Duration::from_millis(300))
            };
            if let Some(command) = shortcut_command(&mode, event.state, held) {
                self.controller.send(command);
            }
        }
        #[cfg(any(target_os = "macos", windows))]
        while let Ok(event) = self.tray_events.try_recv() {
            match event.as_str() {
                "show" => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                "toggle" => self.controller.send("toggle"),
                "quit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                _ => {}
            }
        }
        let status = self.controller.status.lock().unwrap().clone();
        if let Some(manager) = &self.hotkeys {
            if status.phase == "listening" && self.cancel_hotkey.is_none() {
                let escape = HotKey::new(None, global_hotkey::hotkey::Code::Escape);
                if manager.register(escape).is_ok() {
                    self.cancel_hotkey = Some(escape);
                }
            } else if status.phase != "listening" {
                if let Some(escape) = self.cancel_hotkey.take() {
                    let _ = manager.unregister(escape);
                }
            }
        }
        if status.phase != "idle"
            && self
                .controller
                .settings
                .lock()
                .unwrap()
                .bool("show_overlay")
        {
            let controller = self.controller.clone();
            let phase = status.phase;
            let level = status.level;
            let monitor = ctx
                .input(|input| input.viewport().monitor_size)
                .unwrap_or(egui::vec2(1440.0, 900.0));
            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("recording-overlay"),
                egui::ViewportBuilder::default()
                    .with_title("Bolum recording")
                    .with_inner_size([290.0, 76.0])
                    .with_decorations(false)
                    .with_resizable(false)
                    .with_taskbar(false)
                    .with_active(false)
                    .with_always_on_top()
                    .with_position([monitor.x / 2.0 - 145.0, monitor.y - 120.0]),
                move |ctx, _| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(if phase == "listening" {
                                "Listening…"
                            } else {
                                "Transcribing…"
                            });
                            if phase == "listening" && ui.button("Finish").clicked() {
                                controller.send("stop");
                            }
                        });
                        if phase == "listening" {
                            ui.add(
                                egui::ProgressBar::new((level * 5.0).min(1.0))
                                    .text("Esc to cancel"),
                            );
                        }
                    });
                },
            );
        }
        if self.last_transcript != status.transcript {
            self.view.copied = false;
            self.last_transcript = status.transcript.clone();
        }
        #[cfg(any(target_os = "macos", windows))]
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(format!("Bolum — {}", status.phase)));
        }
        let mut screen = Screen {
            accessibility_ready: self.accessibility_ready,
            settings: self.controller.settings.lock().unwrap().clone(),
            status,
            microphones: self.microphones.clone(),
            outputs: self.outputs.clone(),
            models: models::CATALOG
                .iter()
                .map(|model| ModelView {
                    id: model.id,
                    downloaded: models::is_downloaded(model),
                    downloading: self.downloads.is_downloading(model.id),
                    progress: self
                        .progress
                        .lock()
                        .unwrap()
                        .get(model.id)
                        .copied()
                        .unwrap_or(0.0),
                })
                .collect(),
            #[cfg(any(target_os = "macos", windows))]
            tray_available: self.tray.is_some(),
            #[cfg(not(any(target_os = "macos", windows)))]
            tray_available: false,
        };
        for action in self.view.show(ctx, &mut screen) {
            self.action(action, ctx);
        }
    }
}
#[cfg(any(target_os = "macos", windows))]
fn create_tray(ctx: egui::Context) -> (Option<tray_icon::TrayIcon>, mpsc::Receiver<String>) {
    use tray_icon::{
        menu::{Menu, MenuEvent, MenuItem},
        Icon, TrayIconBuilder,
    };
    let (tx, rx) = mpsc::channel();
    let result = (|| -> Result<tray_icon::TrayIcon> {
        let menu = Menu::new();
        for (id, title) in [
            ("show", "Open Bolum"),
            ("toggle", "Start / stop dictation"),
            ("quit", "Quit"),
        ] {
            menu.append(&MenuItem::with_id(id, title, true, None))?;
        }
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let _ = tx.send(event.id.0);
            ctx.request_repaint();
        }));
        // Tiny monochrome microphone icon, generated without an image decoder.
        let mut rgba = vec![0; 22 * 22 * 4];
        for y in 3..19 {
            for x in 5..17 {
                if (8..14).contains(&x) && y < 13
                    || (y == 14 && (6..16).contains(&x))
                    || (x == 10 && y > 14)
                    || (y == 18 && (7..15).contains(&x))
                {
                    let i = (y * 22 + x) * 4;
                    rgba[i..i + 4].copy_from_slice(&[100, 100, 100, 255]);
                }
            }
        }
        Ok(TrayIconBuilder::new()
            .with_tooltip("Bolum")
            .with_menu(Box::new(menu))
            .with_icon(Icon::from_rgba(rgba, 22, 22)?)
            .with_icon_as_template(cfg!(target_os = "macos"))
            .build()?)
    })();
    if let Err(err) = &result {
        log::warn!("Tray unavailable: {err}");
    }
    (result.ok(), rx)
}
