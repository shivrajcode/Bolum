//! Presentation and user intents, separated from OS services for interaction tests.
use crate::{models, pipeline::Status, settings::SettingsStore};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

const ACCENT: Color32 = Color32::from_rgb(105, 75, 167);
const LAVENDER: Color32 = Color32::from_rgb(232, 222, 249);
const INK: Color32 = Color32::from_rgb(42, 38, 51);
const SECONDARY: Color32 = Color32::from_rgb(107, 101, 117);
const BACKGROUND: Color32 = Color32::from_rgb(249, 248, 251);

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Page {
    #[default]
    General,
    Models,
    Audio,
    Transcript,
    About,
}
impl Page {
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Models,
        Self::Audio,
        Self::Transcript,
        Self::About,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Models => "Speech models",
            Self::Audio => "Audio input",
            Self::Transcript => "Last dictation",
            Self::About => "About & permissions",
        }
    }
    fn subtitle(self) -> &'static str {
        match self {
            Self::General => "A little less typing. A little more flow.",
            Self::Models => "Choose the voice model that feels right for you.",
            Self::Audio => "Make sure your voice comes through clearly.",
            Self::Transcript => "Your most recent words, kept only for this session.",
            Self::About => "A quiet place for your voice. Entirely on your device.",
        }
    }
}
pub struct ModelView {
    pub id: &'static str,
    pub downloaded: bool,
    pub downloading: bool,
    pub progress: f32,
}
pub struct Screen {
    pub settings: SettingsStore,
    pub status: Status,
    pub microphones: Vec<String>,
    pub models: Vec<ModelView>,
    pub tray_available: bool,
}
pub enum Action {
    Save(SettingsStore),
    ApplyShortcut(String),
    Command(&'static str),
    Download(&'static str),
    CancelDownload(&'static str),
    Delete(&'static str),
    RefreshMicrophones,
    Copy,
    ClearTranscript,
    Dismiss,
    Hide,
    Quit,
    #[cfg(target_os = "macos")]
    Permissions(&'static str),
}
#[derive(Default)]
pub struct View {
    pub page: Page,
    pub hotkey_text: String,
    pub shortcut_error: String,
    pub(super) pending_delete: Option<&'static str>,
    pub copied: bool,
}

pub fn configure(ctx: &egui::Context, system_font: bool) {
    ctx.set_theme(egui::Theme::Light);
    let mut style = egui::Style {
        visuals: egui::Visuals::light(),
        ..Default::default()
    };
    style.visuals.override_text_color = Some(INK);
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = Color32::WHITE;
    style.visuals.faint_bg_color = Color32::from_rgb(244, 240, 249);
    style.visuals.extreme_bg_color = Color32::from_rgb(251, 250, 253);
    style.visuals.selection.bg_fill = LAVENDER;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(241, 237, 247);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(241, 237, 247);
    style.visuals.widgets.inactive.bg_stroke =
        Stroke::new(1.0_f32, Color32::from_rgb(225, 220, 233));
    style.visuals.widgets.hovered.weak_bg_fill = LAVENDER;
    style.visuals.widgets.hovered.bg_fill = LAVENDER;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.5));
    style.visuals.widgets.active.bg_fill = LAVENDER;
    style.visuals.widgets.active.weak_bg_fill = LAVENDER;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(6);
    }
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.interact_size.y = 30.0;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(25.0));
    ctx.set_style(style);
    if system_font && cfg!(target_os = "macos") {
        if let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNS.ttf") {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "San Francisco".into(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "San Francisco".into());
            ctx.set_fonts(fonts);
        }
    }
}
fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).color(SECONDARY).size(12.0));
}
fn group(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .corner_radius(10)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(232, 228, 238)))
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            contents(ui);
        });
}
fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(Color32::WHITE))
            .fill(ACCENT)
            .corner_radius(7),
    )
}
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong().size(13.0));
}
fn choice(ui: &mut egui::Ui, label: &str, value: &mut String, options: &[(&str, &str)]) -> bool {
    let selected = options
        .iter()
        .find(|(id, _)| *id == value)
        .map_or(value.as_str(), |(_, text)| text)
        .to_owned();
    let mut changed = false;
    ui.horizontal(|ui| {
        let label_response = ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_salt(label)
                .selected_text(selected)
                .width(170.0)
                .show_ui(ui, |ui| {
                    for (id, text) in options {
                        changed |= ui.selectable_value(value, (*id).into(), *text).changed();
                    }
                })
                .response
                .labelled_by(label_response.id);
        });
    });
    changed
}
fn set_bool(ui: &mut egui::Ui, settings: &mut SettingsStore, key: &str, label: &str) -> bool {
    let mut value = settings.bool(key);
    let changed = ui.checkbox(&mut value, label).changed();
    if changed {
        settings.set_quiet(key, value.into());
    }
    changed
}
impl View {
    pub fn show(&mut self, ctx: &egui::Context, screen: &mut Screen) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut changed = false;
        let idle = screen.status.phase == "idle";
        ctx.input_mut(|input| {
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::Q) {
                actions.push(Action::Quit);
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::W) {
                actions.push(if screen.tray_available {
                    Action::Hide
                } else {
                    Action::Quit
                });
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma) {
                self.page = Page::General;
            }
            if screen.status.phase == "listening"
                && input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
            {
                actions.push(Action::Command("cancel"));
            }
        });
        egui::SidePanel::left("navigation")
            .exact_width(184.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(241, 237, 248))
                    .inner_margin(16),
            )
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.label(RichText::new("bolum").size(27.0).strong().color(ACCENT));
                muted(ui, "A moment. In your words.");
                ui.add_space(30.0);
                for page in Page::ALL {
                    let selected = self.page == page;
                    let response = ui.add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new(RichText::new(page.label()).color(if selected {
                            ACCENT
                        } else {
                            SECONDARY
                        }))
                        .selected(selected)
                        .frame(selected)
                        .corner_radius(7),
                    );
                    if response.clicked() {
                        self.page = page;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    muted(ui, format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label(
                        RichText::new("On-device dictation")
                            .size(12.0)
                            .color(ACCENT),
                    );
                    ui.add_space(8.0);
                    if ui.small_button("Quit Bolum").clicked() {
                        actions.push(Action::Quit);
                    }
                    if screen.tray_available
                        && ui
                            .small_button(if cfg!(target_os = "macos") {
                                "Hide to menu bar"
                            } else {
                                "Hide to tray"
                            })
                            .clicked()
                    {
                        actions.push(Action::Hide);
                    }
                });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BACKGROUND).inner_margin(28)).show(ctx, |ui| {
            let heading = ui.heading(self.page.label());
            ctx.accesskit_node_builder(heading.id, |node| { node.set_role(egui::accesskit::Role::Heading); node.set_level(1); });
            muted(ui, self.page.subtitle());
            ui.add_space(14.0);
            if !screen.status.notice.is_empty() {
                egui::Frame::new().fill(Color32::from_rgb(255, 244, 229)).inner_margin(12).corner_radius(8).show(ui, |ui| {
                    ui.label(RichText::new(&screen.status.notice).color(Color32::from_rgb(126, 77, 26)));
                    if ui.small_button("Dismiss message").clicked() { actions.push(Action::Dismiss); }
                });
                ui.add_space(8.0);
            }
            // Capture controls stay available on every page, even while settings are locked.
            if !idle {
                group(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(if screen.status.phase == "listening" { "Listening to you…" } else { "Turning speech into text…" }).strong().color(ACCENT));
                        if screen.status.phase == "listening" {
                            if primary(ui, "Finish recording").clicked() { actions.push(Action::Command("stop")); }
                            if ui.button("Cancel recording").clicked() { actions.push(Action::Command("cancel")); }
                        }
                    });
                    if screen.status.phase == "listening" { ui.add(egui::ProgressBar::new((screen.status.level * 5.0).min(1.0)).fill(ACCENT).text("Input level")); }
                    else { muted(ui, "You can keep using other apps. Your transcript will appear shortly."); }
                });
                ui.add_space(10.0);
            }
            egui::ScrollArea::vertical().id_salt(self.page.label()).auto_shrink([false, false]).show(ui, |ui| {
                match self.page {
                    Page::General => changed |= self.general(ui, screen, &mut actions),
                    Page::Models => changed |= self.models(ui, screen, &mut actions),
                    Page::Audio => changed |= self.audio(ui, screen, &mut actions),
                    Page::Transcript => self.transcript(ui, screen, &mut actions),
                    Page::About => self.about(ui, &mut actions),
                }
            });
        });
        if changed {
            actions.insert(0, Action::Save(screen.settings.clone()));
        }
        actions
    }
    fn general(
        &mut self,
        ui: &mut egui::Ui,
        screen: &mut Screen,
        actions: &mut Vec<Action>,
    ) -> bool {
        let mut changed = false;
        let selected = screen.settings.str("model");
        let ready = screen
            .models
            .iter()
            .any(|model| model.id == selected && model.downloaded);
        group(ui, |ui| {
            ui.label(
                RichText::new(if ready {
                    "Your voice, ready to go."
                } else {
                    "Let’s set up your voice."
                })
                .size(20.0)
                .strong(),
            );
            if ready {
                muted(
                    ui,
                    "Click into any text field, then use your shortcut to start dictating.",
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(shortcut_label(&screen.settings.str("hotkey")))
                        .size(20.0)
                        .color(ACCENT),
                );
                if let Ok(model) = models::find(&selected) {
                    muted(ui, model.name);
                }
            } else {
                muted(
                    ui,
                    "Download a speech model once. After that, your words stay on this device.",
                );
                if primary(ui, "Choose a speech model").clicked() {
                    self.page = Page::Models;
                }
            }
        });
        section(ui, "Dictation");
        ui.add_enabled_ui(screen.status.phase == "idle", |ui| {
            group(ui, |ui| {
                ui.label("Keyboard shortcut");
                ui.horizontal(|ui| {
                    let edit = ui.add(egui::TextEdit::singleline(&mut self.hotkey_text).desired_width(230.0).hint_text("ctrl+alt+space"));
                    edit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Keyboard shortcut"));
                    if ui.button("Apply shortcut").clicked() {
                        match self.hotkey_text.parse::<global_hotkey::hotkey::HotKey>() {
                            Ok(_) => { self.shortcut_error.clear(); actions.push(Action::ApplyShortcut(self.hotkey_text.clone())); }
                            Err(_) => self.shortcut_error = "Use a shortcut such as ctrl+alt+space or super+shift+d.".into(),
                        }
                    }
                });
                if !self.shortcut_error.is_empty() { ui.colored_label(Color32::from_rgb(155, 64, 59), &self.shortcut_error); }
                muted(ui, "Use ctrl, alt, shift or super, joined with + and a key.");
                ui.separator();
                changed |= set_bool(ui, &mut screen.settings, "push_to_talk", "Hold shortcut to talk");
                muted(ui, if screen.settings.bool("push_to_talk") { "Release the shortcut to transcribe." } else { "Press once to start. Press again to finish." });
            });
            section(ui, "Text insertion");
            group(ui, |ui| {
                let mut mode = screen.settings.str("injection");
                if choice(ui, "Delivery", &mut mode, &[("clipboard", "Paste into active app"), ("type", "Type into active app"), ("none", "Show in Bolum only")]) {
                    screen.settings.set_quiet("injection", mode.clone().into()); changed = true;
                }
                muted(ui, match mode.as_str() { "clipboard" => "Pasting replaces your clipboard. Accessibility permission is required on macOS.", "type" => "Text is typed directly into the focused field.", _ => "Your words appear in Last dictation. Nothing is inserted into other apps." });
                ui.separator();
                changed |= set_bool(ui, &mut screen.settings, "append_trailing_space", "Add a space after each dictation");
            });
        });
        ui.add_space(8.0);
        muted(
            ui,
            "Changes save automatically. Recordings stop after two minutes.",
        );
        if cfg!(target_os = "linux") {
            muted(
                ui,
                "On Wayland, bind bolum toggle in your compositor to start dictation.",
            );
        }
        changed
    }
    fn models(
        &mut self,
        ui: &mut egui::Ui,
        screen: &mut Screen,
        actions: &mut Vec<Action>,
    ) -> bool {
        let mut changed = false;
        let mut selected = screen.settings.str("model");
        let idle = screen.status.phase == "idle";
        // Lead with the default, then the smaller alternative.
        for id in [
            "parakeet-tdt-0.6b-v3",
            "canary-180m-flash",
            "parakeet-tdt-0.6b-v2",
        ] {
            let model = models::find(id).unwrap();
            let Some(state) = screen.models.iter().find(|entry| entry.id == id) else {
                continue;
            };
            ui.push_id(id, |ui| {
                group(ui, |ui| {
                    ui.label(
                        RichText::new(model.name.trim_start_matches("NVIDIA "))
                            .strong()
                            .size(16.0),
                    );
                    muted(
                        ui,
                        format!(
                            "{}  ·  {} MB download",
                            model.languages,
                            models::total_size(model) / 1_000_000
                        ),
                    );
                    muted(
                        ui,
                        match id {
                            "canary-180m-flash" => {
                                "The smallest download. Choose your spoken language below."
                            }
                            "parakeet-tdt-0.6b-v3" => {
                                "Recommended. Recognizes the language you speak automatically."
                            }
                            _ => "For English dictation with Parakeet v2.",
                        },
                    );
                    if state.downloading {
                        ui.add(
                            egui::ProgressBar::new(state.progress)
                                .fill(ACCENT)
                                .show_percentage(),
                        );
                        if ui.button("Cancel download").clicked() {
                            actions.push(Action::CancelDownload(id));
                        }
                    } else if state.downloaded {
                        ui.horizontal(|ui| {
                            if selected == id {
                                ui.label(RichText::new("Selected model").color(ACCENT).strong());
                            } else if ui
                                .add_enabled(idle, egui::Button::new("Use model"))
                                .clicked()
                            {
                                selected = id.into();
                                changed = true;
                            }
                            if ui.add_enabled(idle, egui::Button::new("Remove…")).clicked() {
                                self.pending_delete = Some(id);
                            }
                        });
                        if self.pending_delete == Some(id) {
                            ui.separator();
                            ui.label("Remove this download? You can download it again later.");
                            ui.horizontal(|ui| {
                                if ui.button("Keep model").clicked() {
                                    self.pending_delete = None;
                                }
                                if ui
                                    .add_enabled(idle, egui::Button::new("Delete download"))
                                    .clicked()
                                {
                                    actions.push(Action::Delete(id));
                                    self.pending_delete = None;
                                }
                            });
                        }
                    } else if primary(ui, "Download model").clicked() {
                        actions.push(Action::Download(id));
                    }
                    if id == "canary-180m-flash" && selected == id {
                        ui.add_enabled_ui(idle, |ui| {
                            ui.scope(|ui| {
                                let mut language = screen.settings.str("language");
                                if choice(
                                    ui,
                                    "Spoken language",
                                    &mut language,
                                    &[
                                        ("en", "English"),
                                        ("de", "German"),
                                        ("es", "Spanish"),
                                        ("fr", "French"),
                                    ],
                                ) {
                                    screen.settings.set_quiet("language", language.into());
                                    changed = true;
                                }
                            });
                        });
                    }
                });
            });
            ui.add_space(4.0);
        }
        screen.settings.set_quiet("model", selected.clone().into());
        muted(
            ui,
            "Weights are stored separately from the app and verified after download.",
        );
        changed
    }
    fn audio(&mut self, ui: &mut egui::Ui, screen: &mut Screen, actions: &mut Vec<Action>) -> bool {
        let mut changed = false;
        let idle = screen.status.phase == "idle";
        ui.add_enabled_ui(idle, |ui| {
            section(ui, "Input device");
            group(ui, |ui| {
                let mut microphone = screen.settings.str("microphone");
                let mut devices = vec![(String::new(), "System default".to_string())];
                devices.extend(screen.microphones.iter().map(|name| (name.clone(), name.clone())));
                let options: Vec<_> = devices.iter().map(|(id, name)| (id.as_str(), name.as_str())).collect();
                if choice(ui, "Microphone", &mut microphone, &options) { screen.settings.set_quiet("microphone", microphone.into()); changed = true; }
                if screen.microphones.is_empty() { muted(ui, "No input devices found. Connect a microphone, allow access, then refresh."); }
                if ui.button("Refresh devices").clicked() { actions.push(Action::RefreshMicrophones); }
            });
            section(ui, "Try your microphone");
            group(ui, |ui| {
                ui.label("Say a sentence. See it here.");
                muted(ui, "This test shows your transcript in Bolum without typing or pasting into another app.");
                let ready = screen.models.iter().any(|model| model.id == screen.settings.str("model") && model.downloaded);
                if ui.add_enabled(ready, egui::Button::new("Record a sample").fill(LAVENDER)).clicked() { self.copied = false; actions.push(Action::Command("start_preview")); }
                if !ready { muted(ui, "Download and select a speech model first."); }
            });
        });
        if !screen.status.transcript.is_empty() {
            section(ui, "Latest result");
            group(ui, |ui| {
                ui.label(&screen.status.transcript);
            });
        }
        #[cfg(target_os = "macos")]
        {
            ui.add_space(8.0);
            if ui.button("Open microphone permissions").clicked() {
                actions.push(Action::Permissions("microphone"));
            }
        }
        changed
    }
    fn transcript(&mut self, ui: &mut egui::Ui, screen: &Screen, actions: &mut Vec<Action>) {
        if screen.status.transcript.is_empty() {
            group(ui, |ui| {
                ui.label(
                    RichText::new("Your next thought starts here.")
                        .size(19.0)
                        .strong(),
                );
                muted(ui, "Dictate with your shortcut, or record a sample in Audio input. Your words will appear here.");
                if ui.button("Try your microphone").clicked() {
                    self.page = Page::Audio;
                }
            });
        } else {
            group(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&screen.status.transcript).size(17.0))
                        .selectable(true)
                        .wrap(),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    if primary(ui, if self.copied { "Copied" } else { "Copy text" }).clicked() {
                        actions.push(Action::Copy);
                    }
                    if ui.button("Clear transcript").clicked() {
                        self.copied = false;
                        actions.push(Action::ClearTranscript);
                    }
                });
            });
        }
        ui.add_space(8.0);
        muted(
            ui,
            "Only the latest transcript is kept in memory. Quitting Bolum clears it.",
        );
    }
    fn about(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        group(ui, |ui| {
            ui.label(RichText::new("Bolum").size(24.0).strong().color(ACCENT));
            muted(
                ui,
                format!(
                    "Version {} · Made for everyday dictation",
                    env!("CARGO_PKG_VERSION")
                ),
            );
            ui.label("Speak naturally. Your computer does the writing.");
            muted(ui, "Speech is processed locally using NVIDIA Parakeet or Canary. No recordings or transcripts are saved to disk.");
        });
        section(ui, "Permissions");
        group(ui, |ui| {
            ui.strong("Microphone");
            muted(ui, "Allows Bolum to hear you while a recording is active.");
            #[cfg(target_os = "macos")]
            if ui.button("Microphone settings").clicked() {
                actions.push(Action::Permissions("microphone"));
            }
            ui.separator();
            ui.strong("Accessibility");
            muted(ui, "Allows Bolum to insert words into another app. You can use Show in Bolum only without text insertion access.");
            #[cfg(target_os = "macos")]
            if ui.button("Accessibility settings").clicked() {
                actions.push(Action::Permissions("accessibility"));
            }
        });
        #[cfg(not(target_os = "macos"))]
        let _ = actions;
        ui.add_space(8.0);
        muted(
            ui,
            "Inspired by Handy. Built in Rust. NVIDIA models: CC-BY-4.0. Application: MIT.",
        );
    }
}
fn shortcut_label(value: &str) -> String {
    value
        .split('+')
        .map(|key| match key.trim().to_lowercase().as_str() {
            "ctrl" | "control" => "Control".into(),
            "alt" | "option" => "Option".into(),
            "super" | "meta" | "cmd" => "Command".into(),
            "shift" => "Shift".into(),
            "space" => "Space".into(),
            _ => key.to_uppercase(),
        })
        .collect::<Vec<_>>()
        .join("  +  ")
}
