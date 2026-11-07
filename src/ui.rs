//! Native egui settings window, with no browser, JavaScript, or web server.
use crate::{audio, inject, models, pipeline::Controller};
use anyhow::Result;
use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

pub fn run() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 640.0])
            .with_min_inner_size([420.0, 400.0]),
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
    hotkey_text: String,
    events: mpsc::Receiver<GlobalHotKeyEvent>,
    #[cfg(any(target_os = "macos", windows))]
    tray: Option<tray_icon::TrayIcon>,
    #[cfg(any(target_os = "macos", windows))]
    tray_events: mpsc::Receiver<String>,
}
impl Desktop {
    fn new(cc: &eframe::CreationContext<'_>) -> Result<Self> {
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
            hotkey_text,
            events,
            #[cfg(any(target_os = "macos", windows))]
            tray,
            #[cfg(any(target_os = "macos", windows))]
            tray_events,
        };
        app.apply_hotkey();
        if let Err(err) = inject::request_permissions() {
            app.controller.notice(err.to_string());
        }
        Ok(app)
    }
    fn apply_hotkey(&mut self) {
        let result = (|| -> Result<()> {
            let manager = self
                .hotkeys
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Global shortcuts unavailable; use bolum toggle"))?;
            let next: HotKey = self.hotkey_text.parse()?;
            if self.hotkey == Some(next) {
                return Ok(());
            }
            // Keep the old shortcut registered if the replacement fails.
            manager.register(next)?;
            if let Some(old) = self.hotkey.replace(next) {
                let _ = manager.unregister(old);
            }
            let mut settings = self.controller.settings.lock().unwrap();
            settings.set_quiet("hotkey", self.hotkey_text.clone().into());
            settings.save()?;
            Ok(())
        })();
        if let Err(err) = result {
            self.controller.notice(format!("Shortcut: {err}"));
        }
    }
    fn download(&self, id: &str, ctx: egui::Context) {
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
        let idle = status.phase == "idle";
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "Bolum — {}",
            status.phase
        )));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Bolum");
            ui.label("Local voice dictation");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.strong(match status.phase { "listening" => "Listening…", "working" => "Transcribing…", _ => "Ready" });
                if status.phase == "listening" {
                    if ui.button("Stop").clicked() { self.controller.send("stop"); }
                    if ui.button("Cancel").clicked() { self.controller.send("cancel"); }
                }
            });
            if status.phase == "listening" { ui.add(egui::ProgressBar::new((status.level * 5.0).min(1.0)).text("Microphone")); }
            if !status.notice.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(190, 95, 45), &status.notice);
                if ui.small_button("Dismiss").clicked() { self.controller.notice(""); }
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_enabled_ui(idle, |ui| {
                    let mut settings = self.controller.settings.lock().unwrap().clone();
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Shortcut"); ui.text_edit_singleline(&mut self.hotkey_text);
                        if ui.button("Apply").clicked() { self.apply_hotkey(); }
                    });
                    let mut hold = settings.bool("push_to_talk");
                    if ui.checkbox(&mut hold, "Hold shortcut to talk (otherwise press to toggle)").changed() { settings.set_quiet("push_to_talk", hold.into()); changed = true; }
                    let mut microphone = settings.str("microphone");
                    egui::ComboBox::from_label("Microphone").selected_text(if microphone.is_empty() { "System default" } else { &microphone }).show_ui(ui, |ui| {
                        changed |= ui.selectable_value(&mut microphone, String::new(), "System default").changed();
                        for device in &self.microphones { changed |= ui.selectable_value(&mut microphone, device.clone(), device).changed(); }
                    });
                    settings.set_quiet("microphone", microphone.into());
                    if ui.small_button("Refresh microphones").clicked() { self.microphones = audio::input_devices(); }
                    let mut model_id = settings.str("model");
                    ui.add_space(12.0); ui.strong("Models");
                    for model in models::CATALOG {
                        let downloaded = models::is_downloaded(model);
                        let downloading = self.downloads.is_downloading(model.id);
                        ui.group(|ui| {
                            ui.label(model.name);
                            ui.small(format!("{} · {} MB", model.languages, models::total_size(model) / 1_000_000));
                            ui.horizontal(|ui| {
                                if downloaded && !downloading {
                                    changed |= ui.selectable_value(&mut model_id, model.id.into(), "Use this model").changed();
                                    if ui.small_button("Delete").clicked() {
                                        self.controller.send(&format!("delete:{}", model.id));
                                    }
                                } else if downloading {
                                    let progress = self.progress.lock().unwrap().get(model.id).copied().unwrap_or(0.0);
                                    ui.add(egui::ProgressBar::new(progress).show_percentage());
                                    if ui.small_button("Cancel").clicked() { self.downloads.cancel(model.id); }
                                } else if ui.button("Download").clicked() { self.download(model.id, ctx.clone()); }
                            });
                        });
                    }
                    settings.set_quiet("model", model_id.clone().into());
                    if model_id == "canary-180m-flash" {
                        ui.label("Canary needs an explicit spoken language:");
                        let mut language = settings.str("language");
                        egui::ComboBox::from_label("Language").selected_text(&language).show_ui(ui, |ui| {
                            for code in ["en", "de", "es", "fr"] { changed |= ui.selectable_value(&mut language, code.into(), code).changed(); }
                        });
                        settings.set_quiet("language", language.into());
                    }
                    let mut injection = settings.str("injection");
                    egui::ComboBox::from_label("Insert text").selected_text(&injection).show_ui(ui, |ui| {
                        for (mode, label) in [("clipboard", "Paste (replaces clipboard)"), ("type", "Type"), ("none", "Show transcript only")] {
                            changed |= ui.selectable_value(&mut injection, mode.into(), label).changed();
                        }
                    });
                    settings.set_quiet("injection", injection.into());
                    if changed {
                        if let Err(err) = settings.save() { self.controller.notice(format!("Settings: {err}")); }
                        *self.controller.settings.lock().unwrap() = settings;
                    }
                });
                ui.add_space(12.0);
                ui.label("Focus a text field in another app, then use your shortcut.");
                ui.small("Wayland: bind bolum toggle in your compositor. Models download once; dictation stays local.");
                if !status.transcript.is_empty() {
                    ui.separator(); ui.strong("Last transcript");
                    ui.label(&status.transcript);
                    if ui.button("Copy").clicked() {
                        if let Err(err) = inject::copy_to_clipboard(&status.transcript) { self.controller.notice(err.to_string()); }
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    #[cfg(any(target_os = "macos", windows))]
                    if self.tray.is_some() && ui.button("Hide to tray").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false)); }
                    if ui.button("Quit").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                });
            });
        });
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
