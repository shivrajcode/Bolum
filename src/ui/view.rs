//! Handy-inspired settings, rendered entirely in Rust.
use crate::{models, pipeline::Status, settings::SettingsStore};
use eframe::egui::{self, Color32, FontId, RichText, Stroke, Vec2};
use egui_phosphor::regular as icon;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Page {
    #[default]
    General,
    History,
    Models,
    Advanced,
    About,
}
impl Page {
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::History,
        Self::Models,
        Self::Advanced,
        Self::About,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::History => "History",
            Self::Models => "Models",
            Self::Advanced => "Advanced",
            Self::About => "About",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::General => icon::HAND_WAVING,
            Self::History => icon::CLOCK_COUNTER_CLOCKWISE,
            Self::Models => icon::CPU,
            Self::Advanced => icon::GEAR,
            Self::About => icon::INFO,
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
    pub outputs: Vec<String>,
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
    CopyEntry(String),
    DeleteEntry(u64),
    ClearHistory,
    ClearTranscript,
    Dismiss,
    Hide,
    Quit,
    OpenData,
    OpenSource,
    #[cfg(target_os = "macos")]
    Permissions(&'static str),
}
#[derive(Default)]
pub struct View {
    pub page: Page,
    pub hotkey_text: String,
    pub shortcut_error: String,
    pub copied: bool,
    pub(super) pending_delete: Option<&'static str>,
    pub capturing: bool,
    pub search: String,
    pub language_filter: String,
    pub(super) theme_applied: Option<bool>,
}
#[derive(Clone, Copy)]
struct Palette {
    bg: Color32,
    sidebar: Color32,
    card: Color32,
    input: Color32,
    border: Color32,
    ink: Color32,
    muted: Color32,
    accent: Color32,
    selected: Color32,
}
impl Palette {
    fn new(dark: bool) -> Self {
        let rgb = Color32::from_rgb;
        if dark {
            Self {
                bg: rgb(40, 39, 42),
                sidebar: rgb(37, 36, 39),
                card: rgb(42, 41, 44),
                input: rgb(51, 49, 54),
                border: rgb(63, 61, 67),
                ink: rgb(242, 240, 246),
                muted: rgb(163, 158, 173),
                accent: rgb(192, 157, 241),
                selected: rgb(124, 91, 163),
            }
        } else {
            Self {
                bg: rgb(249, 248, 251),
                sidebar: rgb(241, 238, 247),
                card: Color32::WHITE,
                input: rgb(247, 244, 251),
                border: rgb(224, 218, 233),
                ink: rgb(45, 40, 54),
                muted: rgb(112, 104, 126),
                accent: rgb(113, 75, 166),
                selected: rgb(142, 107, 186),
            }
        }
    }
}
pub fn configure(ctx: &egui::Context, system_font: bool) {
    let mut fonts = egui::FontDefinitions::default();
    if system_font && cfg!(target_os = "macos") {
        for (name, path) in [
            ("system", "/System/Library/Fonts/SFNS.ttf"),
            ("brand", "/System/Library/Fonts/SFNSRounded.ttf"),
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.into(), egui::FontData::from_owned(bytes).into());
                if name == "system" {
                    fonts
                        .families
                        .get_mut(&egui::FontFamily::Proportional)
                        .unwrap()
                        .insert(0, name.into());
                } else {
                    fonts
                        .families
                        .insert(egui::FontFamily::Name("brand".into()), vec![name.into()]);
                }
            }
        }
    }
    if !fonts
        .families
        .contains_key(&egui::FontFamily::Name("brand".into()))
    {
        fonts.families.insert(
            egui::FontFamily::Name("brand".into()),
            fonts.families[&egui::FontFamily::Proportional].clone(),
        );
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}
fn apply_theme(ctx: &egui::Context, dark: bool) {
    let p = Palette::new(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.override_text_color = Some(p.ink);
    visuals.panel_fill = p.bg;
    visuals.window_fill = p.card;
    visuals.extreme_bg_color = p.input;
    visuals.faint_bg_color = p.input;
    visuals.selection.bg_fill = p.selected;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(6);
        widget.bg_fill = p.input;
        widget.weak_bg_fill = p.input;
        widget.bg_stroke = Stroke::new(1.0_f32, p.border);
        widget.fg_stroke = Stroke::new(1.0_f32, p.ink);
    }
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, p.accent);
    let mut style = egui::Style {
        visuals,
        ..Default::default()
    };
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.interact_size.y = 32.0;
    for (kind, size) in [
        (egui::TextStyle::Body, 15.0),
        (egui::TextStyle::Button, 14.0),
        (egui::TextStyle::Small, 12.0),
        (egui::TextStyle::Heading, 25.0),
    ] {
        style.text_styles.insert(kind, FontId::proportional(size));
    }
    ctx.set_style(style);
}
fn muted(ui: &mut egui::Ui, p: Palette, text: impl Into<String>) {
    ui.label(RichText::new(text).color(p.muted).size(13.0));
}
fn section(ui: &mut egui::Ui, p: Palette, title: &str) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .color(p.muted)
            .size(12.0)
            .strong(),
    );
}
fn group(ui: &mut egui::Ui, p: Palette, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(18, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            contents(ui);
        });
}
fn divider(ui: &mut egui::Ui, p: Palette) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0_f32, p.border),
    );
}
fn row(
    ui: &mut egui::Ui,
    p: Palette,
    label: &str,
    help: &str,
    controls: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(width, 40.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new((width * 0.43).clamp(150.0, 270.0), 38.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(label);
                    if !help.is_empty() {
                        ui.label(RichText::new(icon::INFO).color(p.muted).size(15.0))
                            .on_hover_text(help);
                    }
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), controls);
        },
    );
}
fn combo(ui: &mut egui::Ui, label: &str, value: &mut String, options: &[(&str, &str)]) -> bool {
    let selected = options
        .iter()
        .find(|(id, _)| *id == value)
        .map_or(value.as_str(), |(_, text)| text)
        .to_owned();
    let mut changed = false;
    let response = egui::ComboBox::from_id_salt(label)
        .selected_text(selected)
        .width(ui.available_width().min(240.0))
        .show_ui(ui, |ui| {
            for (id, text) in options {
                changed |= ui.selectable_value(value, (*id).into(), *text).changed();
            }
        })
        .response;
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), label)
    });
    changed
}
fn choice(
    ui: &mut egui::Ui,
    p: Palette,
    settings: &mut SettingsStore,
    label: &str,
    key: &str,
    help: &str,
    options: &[(&str, &str)],
) -> bool {
    let mut changed = false;
    let mut value = settings.str(key);
    row(ui, p, label, help, |ui| {
        changed = combo(ui, label, &mut value, options)
    });
    if changed {
        settings.set_quiet(key, value.into());
    }
    changed
}
fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str, p: Palette) -> bool {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(42.0, 25.0), egui::Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, label)
    });
    let t = ui.ctx().animate_bool_with_time(response.id, *value, 0.12);
    ui.painter()
        .rect_filled(rect, 13, if *value { p.selected } else { p.border });
    ui.painter().circle_filled(
        egui::pos2(
            egui::lerp(rect.left() + 12.5..=rect.right() - 12.5, t),
            rect.center().y,
        ),
        10.0,
        Color32::WHITE,
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            15,
            Stroke::new(1.0_f32, p.accent),
            egui::StrokeKind::Outside,
        );
    }
    response.changed()
}
fn bool_row(
    ui: &mut egui::Ui,
    p: Palette,
    settings: &mut SettingsStore,
    label: &str,
    key: &str,
    help: &str,
) -> bool {
    let mut value = settings.bool(key);
    let mut changed = false;
    row(ui, p, label, help, |ui| {
        changed = toggle(ui, &mut value, label, p)
    });
    if changed {
        settings.set_quiet(key, value.into());
    }
    changed
}
fn accent_button(ui: &mut egui::Ui, text: &str, p: Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(Color32::WHITE))
            .fill(p.selected)
            .corner_radius(7),
    )
}
fn icon_button(ui: &mut egui::Ui, glyph: &str, name: &str) -> egui::Response {
    let response = ui.add(egui::Button::new(RichText::new(glyph).size(19.0)).frame(false));
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name));
    response.on_hover_text(name)
}
impl View {
    pub fn show(&mut self, ctx: &egui::Context, screen: &mut Screen) -> Vec<Action> {
        let preference = screen.settings.str("theme");
        let dark = match preference.as_str() {
            "light" => false,
            "dark" => true,
            _ => ctx.system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
        };
        if self.theme_applied != Some(dark) {
            apply_theme(ctx, dark);
            self.theme_applied = Some(dark);
        }
        let p = Palette::new(dark);
        let mut actions = Vec::new();
        let mut changed = false;
        let idle = screen.status.phase == "idle";
        if self.capturing {
            let events = ctx.input(|input| input.events.clone());
            for event in events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    if key == egui::Key::Escape {
                        self.capturing = false;
                        break;
                    }
                    let mut keys = Vec::new();
                    if modifiers.ctrl {
                        keys.push("ctrl".to_owned());
                    }
                    if modifiers.alt {
                        keys.push("alt".to_owned());
                    }
                    if modifiers.shift {
                        keys.push("shift".to_owned());
                    }
                    if modifiers.mac_cmd || (modifiers.command && !modifiers.ctrl) {
                        keys.push("super".to_owned());
                    }
                    if keys.is_empty() {
                        self.shortcut_error = "Include Control, Option, Shift or Command.".into();
                        continue;
                    }
                    keys.push(format!("{key:?}"));
                    let accelerator = keys.join("+");
                    if accelerator.parse::<global_hotkey::hotkey::HotKey>().is_ok() {
                        self.hotkey_text = accelerator.clone();
                        self.shortcut_error.clear();
                        self.capturing = false;
                        actions.push(Action::ApplyShortcut(accelerator));
                    } else {
                        self.shortcut_error =
                            "That key is not supported. Try a letter or Space.".into();
                    }
                    break;
                }
            }
        } else {
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
        }
        egui::TopBottomPanel::bottom("model-footer")
            .exact_height(38.0)
            .frame(
                egui::Frame::new()
                    .fill(p.sidebar)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .inner_margin(egui::Margin::symmetric(14, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if idle { icon::CIRCLE } else { icon::WAVEFORM })
                            .color(p.accent),
                    );
                    let selected = screen.settings.str("model");
                    let label = models::find(&selected)
                        .map(|m| m.name.trim_start_matches("NVIDIA "))
                        .unwrap_or("Choose a model");
                    if ui
                        .add(
                            egui::Button::new(format!("{label}  {}", icon::CARET_DOWN))
                                .frame(false),
                        )
                        .clicked()
                    {
                        self.page = Page::Models;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        muted(ui, p, format!("v{}", env!("CARGO_PKG_VERSION")));
                        muted(
                            ui,
                            p,
                            if idle {
                                "Processed on this device"
                            } else {
                                screen.status.phase
                            },
                        );
                    });
                });
            });
        egui::SidePanel::left("navigation")
            .exact_width(190.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.sidebar)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .inner_margin(12),
            )
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("bolum")
                            .font(FontId::new(
                                37.0,
                                if ctx.fonts(|fonts| {
                                    fonts
                                        .families()
                                        .contains(&egui::FontFamily::Name("brand".into()))
                                }) {
                                    egui::FontFamily::Name("brand".into())
                                } else {
                                    egui::FontFamily::Proportional
                                },
                            ))
                            .strong()
                            .color(p.accent),
                    );
                });
                ui.add_space(16.0);
                divider(ui, p);
                ui.add_space(7.0);
                for page in Page::ALL {
                    let selected = self.page == page;
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 44.0),
                        egui::Sense::click(),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Button,
                            true,
                            selected,
                            page.label(),
                        )
                    });
                    if selected || response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            8,
                            if selected { p.selected } else { p.input },
                        );
                    }
                    let color = if selected { Color32::WHITE } else { p.ink };
                    ui.painter().text(
                        egui::pos2(rect.left() + 12.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        page.icon(),
                        FontId::proportional(24.0),
                        color,
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 48.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        page.label(),
                        FontId::proportional(16.0),
                        color,
                    );
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            rect.expand(1.0),
                            8,
                            Stroke::new(1.0_f32, p.accent),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if response.clicked() {
                        self.page = page;
                        self.capturing = false;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
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
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(28))
            .show(ctx, |ui| {
                if !screen.status.notice.is_empty() {
                    group(ui, p, |ui| {
                        ui.label(&screen.status.notice);
                        if ui.small_button("Dismiss message").clicked() {
                            actions.push(Action::Dismiss);
                        }
                    });
                    ui.add_space(8.0);
                }
                if !idle {
                    group(ui, p, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(if screen.status.phase == "listening" {
                                    "Listening to you…"
                                } else {
                                    "Transcribing…"
                                })
                                .color(p.accent),
                            );
                            if screen.status.phase == "listening" {
                                if accent_button(ui, "Finish recording", p).clicked() {
                                    actions.push(Action::Command("stop"));
                                }
                                if ui.button("Cancel recording").clicked() {
                                    actions.push(Action::Command("cancel"));
                                }
                            }
                        });
                        if screen.status.phase == "listening" {
                            ui.add(
                                egui::ProgressBar::new((screen.status.level * 5.0).min(1.0))
                                    .fill(p.selected)
                                    .text("Input level"),
                            );
                        }
                    });
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .id_salt(self.page.label())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let heading = ui.heading(match self.page {
                            Page::Models => "Transcription models",
                            Page::History => "Dictation history",
                            _ => self.page.label(),
                        });
                        ctx.accesskit_node_builder(heading.id, |node| {
                            node.set_role(egui::accesskit::Role::Heading);
                            node.set_level(1);
                        });
                        ui.add_space(10.0);
                        match self.page {
                            Page::General => changed |= self.general(ui, p, screen, &mut actions),
                            Page::Models => changed |= self.models(ui, p, screen, &mut actions),
                            Page::Advanced => changed |= self.advanced(ui, p, screen),
                            Page::History => self.history(ui, p, screen, &mut actions),
                            Page::About => changed |= self.about(ui, p, screen, &mut actions),
                        }
                        ui.add_space(10.0);
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
        p: Palette,
        screen: &mut Screen,
        actions: &mut Vec<Action>,
    ) -> bool {
        let mut changed = false;
        let idle = screen.status.phase == "idle";
        let ready = screen
            .models
            .iter()
            .any(|m| m.id == screen.settings.str("model") && m.downloaded);
        if !ready {
            group(ui, p, |ui| {
                ui.label("Download a speech model to start dictating.");
                if accent_button(ui, "Choose a speech model", p).clicked() {
                    self.page = Page::Models;
                }
            });
        }
        ui.add_enabled_ui(idle,|ui| {
            section(ui,p,"General");
            group(ui,p,|ui| {
                row(ui,p,"Transcribe shortcut","Click the shortcut and press the keys you want to use.",|ui| {
                    if icon_button(ui,icon::ARROW_COUNTER_CLOCKWISE,"Reset shortcut").clicked(){self.hotkey_text="ctrl+alt+space".into();actions.push(Action::ApplyShortcut(self.hotkey_text.clone()));}
                    let response=ui.add_sized([ui.available_width().min(290.0),34.0],egui::Button::new(if self.capturing {"Press a shortcut…".into()} else {shortcut_label(&self.hotkey_text)}));
                    response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,true,"Change shortcut"));
                    if response.clicked(){self.capturing=true;self.shortcut_error.clear();}
                });
                if !self.shortcut_error.is_empty(){muted(ui,p,&self.shortcut_error);}
                divider(ui,p);
                changed|=choice(ui,p,&mut screen.settings,"Shortcut behavior","shortcut_mode","Auto: tap to toggle, hold for more than 300 ms to use push-to-talk.",&[("auto","Auto"),("toggle","Press to toggle"),("hold","Hold to talk")]);
                divider(ui,p);
                row(ui,p,"Cancel shortcut","Escape cancels an active recording.",|ui|{ui.label(RichText::new("Escape").monospace());});
            });
            if screen.settings.str("model")=="canary-180m-flash" {
                section(ui,p,"Canary 180M Flash settings");
                group(ui,p,|ui| {
                    changed|=choice(ui,p,&mut screen.settings,"Language","language","Choose the language you speak.",&[("en","English"),("de","German"),("es","Spanish"),("fr","French")]);
                    divider(ui,p);changed|=bool_row(ui,p,&mut screen.settings,"Translate to English","translate_english","Translate German, French or Spanish speech into English with Canary.");
                });
            }
            section(ui,p,"Sound");
            group(ui,p,|ui| {
                let mut options=vec![("","Default")];options.extend(screen.microphones.iter().map(|s|(s.as_str(),s.as_str())));
                changed|=choice(ui,p,&mut screen.settings,"Microphone","microphone","The input device used for dictation.",&options);
                if ui.small_button("Refresh devices").clicked(){actions.push(Action::RefreshMicrophones);}
                if screen.microphones.is_empty(){muted(ui,p,"No microphones found. Connect one and allow microphone access.");}
                divider(ui,p);
                #[cfg(target_os="macos")]
                {changed|=bool_row(ui,p,&mut screen.settings,"Mute while recording","mute_recording","Temporarily mute macOS output and restore its previous state when recording stops.");divider(ui,p);}
                changed|=bool_row(ui,p,&mut screen.settings,"Audio feedback","audio_feedback","Play a short local tone when recording starts and finishes.");
                divider(ui,p);
                ui.add_enabled_ui(screen.settings.bool("audio_feedback"),|ui| {
                    let mut options=vec![("","Default")];options.extend(screen.outputs.iter().map(|s|(s.as_str(),s.as_str())));
                    changed|=choice(ui,p,&mut screen.settings,"Output device","output_device","The device used for feedback tones.",&options);
                    divider(ui,p);
                    let mut volume=screen.settings.usize("feedback_volume").min(100) as u32;
                    row(ui,p,"Volume","Feedback tone volume.",|ui|{let r=ui.add(egui::Slider::new(&mut volume,0..=100).suffix("%"));r.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Slider,ui.is_enabled(),"Feedback volume"));changed|=r.changed();});
                    screen.settings.set_quiet("feedback_volume",volume.into());
                });
            });
        });
        section(ui, p, "Try it out");
        group(ui, p, |ui| {
            ui.label("Say a sentence. See it here.");
            muted(
                ui,
                p,
                "A sample stays in Bolum. It never types or pastes into another app.",
            );
            if ui
                .add_enabled(
                    idle && ready,
                    egui::Button::new("Record a sample").fill(p.selected),
                )
                .clicked()
            {
                self.copied = false;
                actions.push(Action::Command("start_preview"));
            }
            if !screen.status.transcript.is_empty() {
                ui.label(&screen.status.transcript);
            }
        });
        changed
    }
    fn advanced(&mut self, ui: &mut egui::Ui, p: Palette, screen: &mut Screen) -> bool {
        let mut changed = false;
        ui.add_enabled_ui(screen.status.phase=="idle",|ui| {
            section(ui,p,"App");group(ui,p,|ui| {
                ui.add_enabled_ui(screen.tray_available,|ui|{changed|=bool_row(ui,p,&mut screen.settings,"Start hidden","start_hidden","Launch into the menu bar instead of opening the settings window.");});divider(ui,p);
                changed|=bool_row(ui,p,&mut screen.settings,"Show recording overlay","show_overlay","Show a small floating recording indicator without focusing the settings window.");divider(ui,p);
                let mut seconds=screen.settings.usize("unload_seconds").to_string();
                row(ui,p,"Unload model","Release model memory after this period of inactivity.",|ui|changed|=combo(ui,"Unload model",&mut seconds,&[("60","After 1 minute"),("300","After 5 minutes"),("900","After 15 minutes"),("0","Never")]));
                screen.settings.set_quiet("unload_seconds",seconds.parse::<u64>().unwrap_or(60).into());
            });
            section(ui,p,"Output");group(ui,p,|ui| {
                changed|=choice(ui,p,&mut screen.settings,"Paste method","injection","Clipboard mode replaces your text clipboard. Direct typing avoids the clipboard.",&[("clipboard","Clipboard paste"),("type","Direct typing"),("none","Show in Bolum only")]);divider(ui,p);
                changed|=bool_row(ui,p,&mut screen.settings,"Auto submit","auto_submit","Send Enter after insertion. Leave off when dictating drafts or messages you want to review.");
            });
            section(ui,p,"Transcription");group(ui,p,|ui| {
                changed|=bool_row(ui,p,&mut screen.settings,"Voice activity detection","vad","Energy-based silence detection trims quiet edges. It is not a neural speech classifier.");divider(ui,p);
                changed|=bool_row(ui,p,&mut screen.settings,"Remove filler words","remove_fillers","Remove standalone English um and uh tokens from the result.");divider(ui,p);
                changed|=bool_row(ui,p,&mut screen.settings,"Append trailing space","append_trailing_space","Add a space after inserted text.");divider(ui,p);
                ui.label("Custom word corrections");muted(ui,p,"One exact correction per line: spoken phrase=preferred spelling");
                let mut words=screen.settings.str("word_replacements");let response=ui.add(egui::TextEdit::multiline(&mut words).desired_width(f32::INFINITY).desired_rows(2).hint_text("bolam=Bolum"));
                response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::TextEdit,true,"Custom word corrections"));
                if response.changed(){screen.settings.set_quiet("word_replacements",words.into());changed=true;}
            });
            section(ui,p,"History");group(ui,p,|ui| {
                let mut limit=screen.settings.usize("history_limit").to_string();
                row(ui,p,"History limit","Session-only history. No transcripts or recordings are saved to disk.",|ui|changed|=combo(ui,"History limit",&mut limit,&[("0","Off"),("20","20 entries"),("50","50 entries"),("100","100 entries")]));
                screen.settings.set_quiet("history_limit",limit.parse::<u64>().unwrap_or(20).into());
                muted(ui,p,"Audio is never saved. Quitting Bolum clears session history.");
            });
        });
        changed
    }
    fn models(
        &mut self,
        ui: &mut egui::Ui,
        p: Palette,
        screen: &mut Screen,
        actions: &mut Vec<Action>,
    ) -> bool {
        let mut changed = false;
        let mut selected = screen.settings.str("model");
        let idle = screen.status.phase == "idle";
        muted(
            ui,
            p,
            "Select a transcription model or download another. All models run locally on your CPU.",
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon::MAGNIFYING_GLASS).color(p.muted));
            let r = ui.add_sized(
                [ui.available_width(), 34.0],
                egui::TextEdit::singleline(&mut self.search).hint_text("Search models by name…"),
            );
            r.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search models")
            });
        });
        ui.horizontal(|ui| {
            muted(ui, p, "Language");
            combo(
                ui,
                "Filter language",
                &mut self.language_filter,
                &[
                    ("", "All languages"),
                    ("en", "English"),
                    ("multi", "Multilingual"),
                ],
            );
        });
        let query = self.search.trim().to_lowercase();
        let mut matches = 0;
        for downloaded in [true, false] {
            let candidates: Vec<_> = [
                "parakeet-tdt-0.6b-v3",
                "canary-180m-flash",
                "parakeet-tdt-0.6b-v2",
            ]
            .into_iter()
            .filter_map(|id| {
                let state = screen.models.iter().find(|m| m.id == id)?;
                let model = models::find(id).ok()?;
                if state.downloaded != downloaded
                    || !model.name.to_lowercase().contains(&query)
                    || (self.language_filter == "multi" && model.languages == "en")
                {
                    None
                } else {
                    Some((model, state))
                }
            })
            .collect();
            if candidates.is_empty() {
                continue;
            }
            section(
                ui,
                p,
                if downloaded {
                    "Downloaded models"
                } else {
                    "Available to download"
                },
            );
            for (model, state) in candidates {
                matches += 1;
                let id = model.id;
                ui.push_id(id,|ui| {
                    let active=selected==id&&state.downloaded;
                    let frame=egui::Frame::new().fill(if active {if p.bg.r()<100 {Color32::from_rgb(57,46,72)}else{Color32::from_rgb(241,233,252)}}else{p.card}).corner_radius(12).stroke(Stroke::new(if active {2.0_f32}else{1.0_f32},if active {p.selected}else{p.border})).inner_margin(18);
                    frame.show(ui,|ui|{
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui|{
                            ui.label(RichText::new(model.name.trim_start_matches("NVIDIA ")).strong().size(18.0));
                            if active {ui.label(RichText::new(format!("{} Active",icon::CHECK)).color(p.accent));}
                            else if id=="parakeet-tdt-0.6b-v3"{ui.label(RichText::new("Recommended").color(p.accent).size(12.0));}
                        });
                        muted(ui,p,match id {"canary-180m-flash"=>"Small and responsive, with four languages and English translation.","parakeet-tdt-0.6b-v3"=>"Accurate multilingual dictation with automatic language recognition.",_=>"English dictation with the Parakeet v2 model."});
                        divider(ui,p);
                        ui.horizontal_wrapped(|ui|{
                            muted(ui,p,format!("{}  {}",icon::GLOBE,model.languages));muted(ui,p,format!("{}  {} MB",icon::HARD_DRIVES,models::total_size(model)/1_000_000));
                            if state.downloading {if ui.button("Cancel download").clicked(){actions.push(Action::CancelDownload(id));}}
                            else if state.downloaded {
                                if !active&&ui.add_enabled(idle,egui::Button::new("Use model")).clicked(){selected=id.into();changed=true;}
                                if ui.add_enabled(idle,egui::Button::new(RichText::new(format!("{} Delete",icon::TRASH)).color(p.accent)).frame(false)).clicked(){self.pending_delete=Some(id);}
                            }else if accent_button(ui,"Download model",p).clicked(){actions.push(Action::Download(id));}
                        });
                        if state.downloading{ui.add(egui::ProgressBar::new(state.progress).fill(p.selected).show_percentage());}
                        if self.pending_delete==Some(id) {
                            ui.label("Remove this download? It can be downloaded again later.");ui.horizontal(|ui|{
                                if ui.button("Keep model").clicked(){self.pending_delete=None;}
                                if ui.add_enabled(idle,egui::Button::new("Delete download")).clicked(){actions.push(Action::Delete(id));self.pending_delete=None;}
                            });
                        }
                    });
                });
                ui.add_space(6.0);
            }
        }
        if matches == 0 {
            muted(
                ui,
                p,
                "No models match your search. Try another name or language filter.",
            );
        }
        screen.settings.set_quiet("model", selected.into());
        changed
    }
    fn history(
        &mut self,
        ui: &mut egui::Ui,
        p: Palette,
        screen: &Screen,
        actions: &mut Vec<Action>,
    ) {
        muted(
            ui,
            p,
            "Your words stay on this device. Session history is cleared when you quit.",
        );
        if !screen.status.transcript.is_empty() {
            section(ui, p, "Latest dictation");
            group(ui, p, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&screen.status.transcript).size(17.0))
                        .selectable(true)
                        .wrap(),
                );
                ui.horizontal(|ui| {
                    if accent_button(ui, if self.copied { "Copied" } else { "Copy text" }, p)
                        .clicked()
                    {
                        actions.push(Action::Copy);
                    }
                    if ui.button("Clear transcript").clicked() {
                        actions.push(Action::ClearTranscript);
                        self.copied = false;
                    }
                });
            });
        }
        if screen.status.history.is_empty() {
            section(ui, p, "Session history");
            group(ui, p, |ui| {
                ui.label("Your next thought starts here.");
                muted(
                    ui,
                    p,
                    "Dictate with your shortcut or try a microphone sample in General.",
                );
                if ui.button("Try your microphone").clicked() {
                    self.page = Page::General;
                }
            });
        } else {
            section(ui, p, "Session history");
            if ui.button("Clear session history").clicked() {
                actions.push(Action::ClearHistory);
            }
            for entry in &screen.status.history {
                ui.push_id(entry.id, |ui| {
                    group(ui, p, |ui| {
                        ui.label(&entry.text);
                        muted(ui, p, &entry.model);
                        ui.horizontal(|ui| {
                            if ui.button("Copy entry").clicked() {
                                actions.push(Action::CopyEntry(entry.text.clone()));
                            }
                            if ui.button("Delete entry").clicked() {
                                actions.push(Action::DeleteEntry(entry.id));
                            }
                        });
                    });
                });
            }
        }
    }
    fn about(
        &mut self,
        ui: &mut egui::Ui,
        p: Palette,
        screen: &mut Screen,
        actions: &mut Vec<Action>,
    ) -> bool {
        let mut changed = false;
        section(ui, p, "About");
        group(ui, p, |ui| {
            changed|=choice(ui,p,&mut screen.settings,"Application theme","theme","Follow system appearance or choose a light or dark surface. Purple remains the accent.",&[("system","System"),("light","Light"),("dark","Dark")]);
            divider(ui, p);
            row(ui, p, "Version", "", |ui| {
                ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
            });
            divider(ui, p);
            row(
                ui,
                p,
                "Source code",
                "Open the project's source repository.",
                |ui| {
                    if ui.button("View on GitHub").clicked() {
                        actions.push(Action::OpenSource);
                    }
                },
            );
            divider(ui, p);
            ui.label("App data directory");
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(crate::settings::data_dir().display().to_string())
                            .monospace()
                            .size(12.0),
                    )
                    .wrap()
                    .selectable(true),
                );
                if ui.button("Open folder").clicked() {
                    actions.push(Action::OpenData);
                }
            });
        });
        section(ui, p, "Permissions");
        group(ui, p, |ui| {
            row(
                ui,
                p,
                "Microphone",
                "Required to hear you while you dictate.",
                |ui| {
                    #[cfg(target_os = "macos")]
                    if ui.button("Microphone settings").clicked() {
                        actions.push(Action::Permissions("microphone"));
                    }
                    #[cfg(not(target_os = "macos"))]
                    ui.label("System microphone access");
                },
            );
            divider(ui, p);
            row(
                ui,
                p,
                "Accessibility",
                "Required on macOS to insert text into another app.",
                |ui| {
                    #[cfg(target_os = "macos")]
                    if ui.button("Accessibility settings").clicked() {
                        actions.push(Action::Permissions("accessibility"));
                    }
                    #[cfg(not(target_os = "macos"))]
                    ui.label("Native text input");
                },
            );
        });
        section(ui, p, "Acknowledgments");
        group(ui, p, |ui| {
            ui.strong("Local speech recognition");
            muted(ui,p,"NVIDIA Parakeet and Canary models run through ONNX Runtime. Model exports: onnx-asr / istupakov.");
            divider(ui, p);
            ui.strong("Made for your desktop");
            muted(ui,p,"Inspired by Handy. Built in Rust with egui. Icons by Phosphor. App: MIT. NVIDIA models: CC-BY-4.0.");
        });
        changed
    }
}
fn shortcut_label(value: &str) -> String {
    value
        .split('+')
        .map(|key| match key.trim().to_lowercase().as_str() {
            "ctrl" | "control" => "Ctrl".into(),
            "alt" | "option" => {
                if cfg!(target_os = "macos") {
                    "Option".into()
                } else {
                    "Alt".into()
                }
            }
            "super" | "meta" | "cmd" => {
                if cfg!(target_os = "macos") {
                    "Command".into()
                } else {
                    "Super".into()
                }
            }
            "shift" => "Shift".into(),
            "space" => "Space".into(),
            _ => key.to_uppercase(),
        })
        .collect::<Vec<_>>()
        .join(" + ")
}
