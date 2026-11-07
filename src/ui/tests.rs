//! Pointer, keyboard and accessibility checks against the production settings view.
use super::view::{self, Action, ModelView, Page, Screen, View};
use crate::{models, pipeline::Status, settings::SettingsStore, transcript::Entry};
use eframe::egui::{self, accesskit::Role};
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
struct State {
    view: View,
    screen: Screen,
    actions: Vec<Action>,
    configured: bool,
}
fn fixture() -> State {
    let mut settings = SettingsStore::load_defaults();
    settings.set_quiet("injection", "clipboard".into());
    settings.set_quiet("theme", "dark".into());
    State {
        view: View {
            hotkey_text: "ctrl+alt+space".into(),
            ..Default::default()
        },
        screen: Screen {
            settings,
            status: Status {
                phase: "idle",
                ..Default::default()
            },
            microphones: vec!["MacBook Microphone".into(), "USB microphone".into()],
            outputs: vec!["MacBook Speakers".into()],
            models: models::CATALOG
                .iter()
                .map(|m| ModelView {
                    id: m.id,
                    downloaded: false,
                    downloading: false,
                    progress: 0.0,
                })
                .collect(),
            tray_available: true,
        },
        actions: Vec::new(),
        configured: false,
    }
}
fn harness(state: State, size: [f32; 2]) -> Harness<'static, State> {
    Harness::builder()
        .with_size(size)
        .with_os(egui::os::OperatingSystem::Mac)
        .build_state(
            |ctx, state: &mut State| {
                if !state.configured {
                    view::configure(ctx, false);
                    state.configured = true;
                }
                state
                    .actions
                    .extend(state.view.show(ctx, &mut state.screen));
            },
            state,
        )
}
fn click(h: &mut Harness<'_, State>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run();
}
fn choose(h: &mut Harness<'_, State>, label: &str, value: &str) {
    h.get_by_role_and_label(Role::ComboBox, label)
        .scroll_to_me();
    h.run();
    h.get_by_role_and_label(Role::ComboBox, label).click();
    h.run();
    h.get_by_label(value).click();
    h.run();
}
#[test]
fn navigation_setup_and_footer_model_selection() {
    let mut h = harness(fixture(), [1000.0, 780.0]);
    click(&mut h, "Choose a speech model");
    assert_eq!(h.state().view.page, Page::Models);
    for page in Page::ALL {
        click(&mut h, page.label());
        assert_eq!(h.state().view.page, page);
        assert!(h.query_by_role(Role::Heading).is_some());
    }
    h.get_by_label_contains("Parakeet TDT 0.6B v3").click();
    h.run();
    assert_eq!(h.state().view.page, Page::Models);
}
#[test]
fn captures_valid_shortcuts_rejects_plain_keys_and_supports_reset() {
    let mut state = fixture();
    state.screen.models[1].downloaded = true;
    let mut h = harness(state, [1000.0, 780.0]);
    click(&mut h, "Change shortcut");
    h.key_press(egui::Key::D);
    h.run();
    assert!(!h.state().view.shortcut_error.is_empty());
    h.key_combination_modifiers(
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
        &[egui::Key::D],
    );
    h.run();
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a,Action::ApplyShortcut(value) if value=="ctrl+shift+D")));
    click(&mut h, "Reset shortcut");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a,Action::ApplyShortcut(value) if value=="ctrl+alt+space")));
    choose(&mut h, "Shortcut behavior", "Auto");
    assert_eq!(h.state().screen.settings.str("shortcut_mode"), "auto");
    choose(&mut h, "Shortcut behavior", "Hold to talk");
    assert_eq!(h.state().screen.settings.str("shortcut_mode"), "hold");
}
#[test]
fn microphone_and_feedback_controls_persist_settings() {
    let mut state = fixture();
    state.screen.models[1].downloaded = true;
    let mut h = harness(state, [1000.0, 900.0]);
    choose(&mut h, "Microphone", "USB microphone");
    assert_eq!(
        h.state().screen.settings.str("microphone"),
        "USB microphone"
    );
    click(&mut h, "Refresh devices");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::RefreshMicrophones)));
    assert!(h
        .get_by_role_and_label(Role::ComboBox, "Output device")
        .accesskit_node()
        .is_disabled());
    h.get_by_role_and_label(Role::CheckBox, "Audio feedback")
        .click();
    h.run();
    assert!(h.state().screen.settings.bool("audio_feedback"));
    choose(&mut h, "Output device", "MacBook Speakers");
    assert_eq!(
        h.state().screen.settings.str("output_device"),
        "MacBook Speakers"
    );
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a,Action::Save(s) if s.bool("audio_feedback"))));
}
#[test]
fn preview_recording_stop_cancel_and_processing_lock() {
    let mut state = fixture();
    state.screen.models[1].downloaded = true;
    let mut h = harness(state, [1000.0, 1000.0]);
    h.get_by_label("Record a sample").scroll_to_me();
    h.run();
    click(&mut h, "Record a sample");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Command("start_preview"))));
    assert_eq!(h.state().screen.settings.str("injection"), "clipboard");
    h.state_mut().screen.status.phase = "listening";
    h.run();
    click(&mut h, "Finish recording");
    click(&mut h, "Cancel recording");
    for cmd in ["stop", "cancel"] {
        assert!(h
            .state()
            .actions
            .iter()
            .any(|a| matches!(a,Action::Command(c) if *c==cmd)));
    }
    h.state_mut().screen.status.phase = "working";
    h.run();
    assert!(h
        .get_by_label("Change shortcut")
        .accesskit_node()
        .is_disabled());
    click(&mut h, "History");
    assert_eq!(h.state().view.page, Page::History);
}
#[test]
fn model_download_cancellation_selection_and_confirmation() {
    let mut state = fixture();
    state.view.page = Page::Models;
    let mut h = harness(state, [1000.0, 1000.0]);
    h.get_all_by_label("Download model").next().unwrap().click();
    h.run();
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Download("parakeet-tdt-0.6b-v3"))));
    h.state_mut().screen.models[1].downloading = true;
    h.state_mut().screen.models[1].progress = 0.42;
    h.run();
    click(&mut h, "Cancel download");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::CancelDownload("parakeet-tdt-0.6b-v3"))));
    h.state_mut().screen.models[1].downloading = false;
    h.state_mut().screen.models[1].downloaded = true;
    h.run();
    h.get_all_by_label_contains("Delete")
        .next()
        .unwrap()
        .click();
    h.run();
    assert!(!h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Delete(_))));
    click(&mut h, "Keep model");
    assert!(h.query_by_label("Delete download").is_none());
    h.get_all_by_label_contains("Delete")
        .next()
        .unwrap()
        .click();
    h.run();
    click(&mut h, "Delete download");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Delete("parakeet-tdt-0.6b-v3"))));
    h.state_mut().screen.models[1].downloaded = false;
    h.state_mut().screen.models[2].downloaded = true;
    h.run();
    click(&mut h, "Use model");
    assert_eq!(h.state().screen.settings.str("model"), "canary-180m-flash");
    click(&mut h, "General");
    choose(&mut h, "Language", "French");
    assert_eq!(h.state().screen.settings.str("language"), "fr");
    h.get_by_role_and_label(Role::CheckBox, "Translate to English")
        .click();
    h.run();
    assert!(h.state().screen.settings.bool("translate_english"));
}
#[test]
fn model_search_and_language_filters_have_empty_state() {
    let mut state = fixture();
    state.view.page = Page::Models;
    let mut h = harness(state, [1000.0, 780.0]);
    h.get_by_label("Search models").focus();
    h.get_by_label("Search models").type_text("canary");
    h.run();
    assert_eq!(h.get_all_by_label("Download model").count(), 1);
    h.state_mut().view.search = "no matching model".into();
    h.run();
    assert!(h.query_by_label("Download model").is_none());
    h.state_mut().view.search.clear();
    h.run();
    choose(&mut h, "Filter language", "Multilingual");
    assert_eq!(h.get_all_by_label("Download model").count(), 2);
}
#[test]
fn advanced_output_transcription_memory_and_history_settings() {
    let mut state = fixture();
    state.view.page = Page::Advanced;
    let mut h = harness(state, [1000.0, 1400.0]);
    choose(&mut h, "Unload model", "After 5 minutes");
    assert_eq!(h.state().screen.settings.usize("unload_seconds"), 300);
    h.get_by_role_and_label(Role::CheckBox, "Start hidden")
        .click();
    h.run();
    assert!(h.state().screen.settings.bool("start_hidden"));
    h.get_by_role_and_label(Role::CheckBox, "Show recording overlay")
        .click();
    h.run();
    assert!(!h.state().screen.settings.bool("show_overlay"));
    choose(&mut h, "Paste method", "Show in Bolum only");
    assert_eq!(h.state().screen.settings.str("injection"), "none");
    for (label, key) in [
        ("Voice activity detection", "vad"),
        ("Remove filler words", "remove_fillers"),
        ("Auto submit", "auto_submit"),
    ] {
        h.get_by_role_and_label(Role::CheckBox, label)
            .scroll_to_me();
        h.run();
        h.get_by_role_and_label(Role::CheckBox, label).click();
        h.run();
        assert!(h.state().screen.settings.bool(key));
    }
    h.get_by_role_and_label(Role::TextInput, "Custom word corrections")
        .focus();
    h.get_by_role_and_label(Role::TextInput, "Custom word corrections")
        .type_text("bolam=Bolum");
    h.run();
    assert_eq!(
        h.state().screen.settings.str("word_replacements"),
        "bolam=Bolum"
    );
    choose(&mut h, "History limit", "Off");
    assert_eq!(h.state().screen.settings.usize("history_limit"), 0);
}
#[test]
fn history_empty_state_and_copy_delete_clear_actions() {
    let mut state = fixture();
    state.view.page = Page::History;
    let mut h = harness(state, [1000.0, 780.0]);
    click(&mut h, "Try your microphone");
    assert_eq!(h.state().view.page, Page::General);
    h.state_mut().view.page = Page::History;
    h.state_mut().screen.status.transcript = "The quick brown fox.".into();
    h.run();
    click(&mut h, "Copy text");
    click(&mut h, "Clear transcript");
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Copy)));
    h.state_mut().screen.status.history = vec![Entry {
        id: 1,
        text: "My words".into(),
        model: "Canary".into(),
    }];
    h.run();
    click(&mut h, "Copy entry");
    click(&mut h, "Delete entry");
    click(&mut h, "Clear session history");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a,Action::CopyEntry(text) if text=="My words")));
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::DeleteEntry(1))));
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::ClearHistory)));
}
#[test]
fn appearance_permissions_source_and_data_controls() {
    let mut state = fixture();
    state.view.page = Page::About;
    let mut h = harness(state, [1000.0, 1000.0]);
    choose(&mut h, "Application theme", "Light");
    assert!(!h.ctx.style().visuals.dark_mode);
    choose(&mut h, "Application theme", "Dark");
    assert!(h.ctx.style().visuals.dark_mode);
    choose(&mut h, "Application theme", "System");
    assert_eq!(h.state().screen.settings.str("theme"), "system");
    click(&mut h, "View on GitHub");
    click(&mut h, "Open folder");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::OpenSource)));
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::OpenData)));
    #[cfg(target_os = "macos")]
    {
        click(&mut h, "Microphone settings");
        click(&mut h, "Accessibility settings");
        for kind in ["microphone", "accessibility"] {
            assert!(h
                .state()
                .actions
                .iter()
                .any(|a| matches!(a,Action::Permissions(k) if *k==kind)));
        }
    }
}
#[test]
fn window_shortcuts_notices_and_minimum_recording_controls() {
    let mut state = fixture();
    state.screen.status.notice = "Microphone unavailable".into();
    state.screen.status.phase = "listening";
    let mut h = harness(state, [760.0, 560.0]);
    click(&mut h, "Dismiss message");
    for label in ["Finish recording", "Cancel recording"] {
        let rect = h.get_by_label(label).rect();
        assert!(
            rect.max.x <= 760.0 && rect.max.y <= 560.0,
            "{label} is clipped: {rect:?}"
        );
    }
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Command("cancel"))));
    h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::W]);
    h.run();
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Hide)));
    h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::Q]);
    h.run();
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Quit)));
}
#[test]
#[ignore = "renders production screens to PNGs on the local GPU"]
fn render_all_pages_for_visual_review() {
    std::fs::create_dir_all("target/ui-review").unwrap();
    for theme in ["dark", "light"] {
        for (width, height) in [(1000, 780), (760, 560)] {
            for page in Page::ALL {
                let mut state = fixture();
                state.view.page = page;
                state.screen.settings.set_quiet("theme", theme.into());
                state.screen.models[2].downloaded = true;
                state
                    .screen
                    .settings
                    .set_quiet("model", "canary-180m-flash".into());
                if page == Page::History {
                    state.screen.status.history = vec![Entry {
                        id: 1,
                        text:
                            "Let's keep this simple. Speak naturally, and let Bolum do the typing."
                                .into(),
                        model: "Canary 180M Flash".into(),
                    }];
                }
                let mut h = harness(state, [width as f32, height as f32]);
                h.run();
                h.render()
                    .unwrap()
                    .save(format!("target/ui-review/{page:?}-{theme}-{width}.png"))
                    .unwrap();
            }
        }
    }
}

#[test]
fn runtime_shortcut_modes_distinguish_taps_and_holds() {
    use global_hotkey::HotKeyState::{Pressed, Released};
    assert_eq!(
        super::shortcut_command("toggle", Pressed, false),
        Some("toggle")
    );
    assert_eq!(super::shortcut_command("toggle", Released, true), None);
    assert_eq!(
        super::shortcut_command("hold", Pressed, false),
        Some("start")
    );
    assert_eq!(
        super::shortcut_command("hold", Released, false),
        Some("stop")
    );
    assert_eq!(
        super::shortcut_command("auto", Pressed, false),
        Some("toggle")
    );
    assert_eq!(super::shortcut_command("auto", Released, false), None);
    assert_eq!(
        super::shortcut_command("auto", Released, true),
        Some("stop")
    );
}
