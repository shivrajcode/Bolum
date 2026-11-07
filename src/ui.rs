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
            .with_inner_size([820.0, 660.0])
            .with_min_inner_size([720.0, 520.0]),
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
    hotkeys: Option<GlobalHotKeyManager>,
    hotkey: Option<HotKey>,
    view: View,
    events: mpsc::Receiver<GlobalHotKeyEvent>,
    last_transcript: String,
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
            hotkeys,
            hotkey: None,
            view: View {
                hotkey_text,
                ..Default::default()
            },
            events,
            last_transcript: String::new(),
            #[cfg(any(target_os = "macos", windows))]
            tray,
            #[cfg(any(target_os = "macos", windows))]
            tray_events,
        };
        app.apply_hotkey();
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
                Ok(()) => *self.controller.settings.lock().unwrap() = settings,
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
            Action::RefreshMicrophones => self.microphones = audio::input_devices(),
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
impl eframe::App for Desktop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.events.try_recv() {
            if self.hotkey.is_none_or(|key| key.id() != event.id) {
                continue;
            }
            let hold = self
                .controller
                .settings
                .lock()
                .unwrap()
                .bool("push_to_talk");
            match event.state {
                HotKeyState::Pressed => self.controller.send(if hold { "start" } else { "toggle" }),
                HotKeyState::Released if hold => self.controller.send("stop"),
                _ => {}
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
        if self.last_transcript != status.transcript {
            self.view.copied = false;
            self.last_transcript = status.transcript.clone();
        }
        #[cfg(any(target_os = "macos", windows))]
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(format!("Bolum — {}", status.phase)));
        }
        let mut screen = Screen {
            settings: self.controller.settings.lock().unwrap().clone(),
            status,
            microphones: self.microphones.clone(),
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
