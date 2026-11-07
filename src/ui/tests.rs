//! Exercise the production UI with pointer/keyboard events and accessible queries.
use super::view::{self, Action, ModelView, Page, Screen, View};
use crate::{models, pipeline::Status, settings::SettingsStore};
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
            models: models::CATALOG
                .iter()
                .map(|model| ModelView {
                    id: model.id,
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
#[test]
fn sidebar_navigates_every_page_and_setup_has_a_next_step() {
    let mut h = harness(fixture(), [820.0, 660.0]);
    click(&mut h, "Choose a speech model");
    assert_eq!(h.state().view.page, Page::Models);
    for page in Page::ALL {
        click(&mut h, page.label());
        assert_eq!(h.state().view.page, page);
        assert_eq!(
            h.get_by_role(Role::Heading).value().as_deref(),
            Some(page.label())
        );
    }
}
#[test]
fn shortcut_validation_and_auto_saved_preferences() {
    let mut h = harness(fixture(), [820.0, 660.0]);
    h.state_mut().view.hotkey_text = "this is invalid".into();
    h.run();
    click(&mut h, "Apply shortcut");
    assert!(!h.state().view.shortcut_error.is_empty());
    assert!(!h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::ApplyShortcut(_))));
    h.state_mut().view.hotkey_text = "ctrl+shift+d".into();
    h.run();
    click(&mut h, "Apply shortcut");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::ApplyShortcut(key) if key == "ctrl+shift+d")));
    h.get_by_label("Hold shortcut to talk").click();
    h.run();
    assert!(h.state().screen.settings.bool("push_to_talk"));
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Save(s) if s.bool("push_to_talk"))));
    h.get_by_label("Add a space after each dictation").click();
    h.run();
    assert!(!h.state().screen.settings.bool("append_trailing_space"));
}
#[test]
fn delivery_choices_have_accessible_labels_and_save() {
    let mut h = harness(fixture(), [820.0, 660.0]);
    for (label, value) in [
        ("Show in Bolum only", "none"),
        ("Type into active app", "type"),
        ("Paste into active app", "clipboard"),
    ] {
        h.get_by_label("Delivery").click();
        h.run();
        h.get_by_label(label).click();
        h.run();
        assert_eq!(h.state().screen.settings.str("injection"), value);
    }
}
#[test]
fn downloads_cancel_selection_and_delete_require_confirmation() {
    let mut state = fixture();
    state.view.page = Page::Models;
    let mut h = harness(state, [820.0, 720.0]);
    h.get_all_by_label("Download model").next().unwrap().click();
    h.run();
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Download("parakeet-tdt-0.6b-v3"))));
    let model = &mut h.state_mut().screen.models[1]; // Parakeet v3
    model.downloading = true;
    model.progress = 0.42;
    h.run();
    click(&mut h, "Cancel download");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::CancelDownload("parakeet-tdt-0.6b-v3"))));
    let model = &mut h.state_mut().screen.models[1];
    model.downloading = false;
    model.downloaded = true;
    h.run();
    click(&mut h, "Remove…");
    assert!(!h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Delete(_))));
    click(&mut h, "Keep model");
    assert!(h.query_by_label("Delete download").is_none());
    click(&mut h, "Remove…");
    click(&mut h, "Delete download");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Delete("parakeet-tdt-0.6b-v3"))));
    h.state_mut().screen.models[1].downloaded = false;
    h.state_mut().screen.models[2].downloaded = true; // Canary
    h.run();
    click(&mut h, "Use model");
    assert_eq!(h.state().screen.settings.str("model"), "canary-180m-flash");
    h.get_by_label("Spoken language").click();
    h.run();
    h.get_by_label("French").click();
    h.run();
    assert_eq!(h.state().screen.settings.str("language"), "fr");
}
#[test]
fn microphone_refresh_selection_and_preview_recording() {
    let mut state = fixture();
    state.view.page = Page::Audio;
    let mut h = harness(state, [820.0, 660.0]);
    assert!(h
        .get_by_label("Record a sample")
        .accesskit_node()
        .is_disabled());
    click(&mut h, "Refresh devices");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::RefreshMicrophones)));
    h.get_by_label("Microphone").click();
    h.run();
    h.get_by_label("USB microphone").click();
    h.run();
    assert_eq!(
        h.state().screen.settings.str("microphone"),
        "USB microphone"
    );
    h.state_mut().screen.models[1].downloaded = true;
    h.run();
    click(&mut h, "Record a sample");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Command("start_preview"))));
    assert_eq!(h.state().screen.settings.str("injection"), "clipboard");
    h.state_mut().screen.status.phase = "listening";
    h.state_mut().screen.status.level = 0.1;
    h.run();
    click(&mut h, "Finish recording");
    click(&mut h, "Cancel recording");
    for command in ["stop", "cancel"] {
        assert!(h
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, Action::Command(c) if *c == command)));
    }
}
#[test]
fn processing_locks_settings_but_keeps_navigation_available() {
    let mut state = fixture();
    state.screen.status.phase = "working";
    let mut h = harness(state, [820.0, 660.0]);
    assert!(h
        .get_by_label("Apply shortcut")
        .accesskit_node()
        .is_disabled());
    click(&mut h, "Audio input");
    assert!(h
        .get_by_label("Refresh devices")
        .accesskit_node()
        .is_disabled());
    assert!(h.query_by_label("Finish recording").is_none());
    click(&mut h, "Last dictation");
    assert_eq!(h.state().view.page, Page::Transcript);
}
#[test]
fn empty_transcript_recovery_copy_clear_and_errors() {
    let mut state = fixture();
    state.view.page = Page::Transcript;
    let mut h = harness(state, [820.0, 660.0]);
    click(&mut h, "Try your microphone");
    assert_eq!(h.state().view.page, Page::Audio);
    h.state_mut().view.page = Page::Transcript;
    h.state_mut().screen.status.transcript = "The quick brown fox jumps over the lazy dog.".into();
    h.run();
    click(&mut h, "Copy text");
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Copy)));
    click(&mut h, "Clear transcript");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::ClearTranscript)));
    h.state_mut().screen.status.notice = "Microphone is not available".into();
    h.run();
    click(&mut h, "Dismiss message");
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Dismiss)));
}
#[test]
fn window_actions_and_permission_buttons_are_wired() {
    let mut state = fixture();
    state.view.page = Page::About;
    let mut h = harness(state, [820.0, 660.0]);
    click(
        &mut h,
        if cfg!(target_os = "macos") {
            "Hide to menu bar"
        } else {
            "Hide to tray"
        },
    );
    click(&mut h, "Quit Bolum");
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Hide)));
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Quit)));
    #[cfg(target_os = "macos")]
    {
        click(&mut h, "Microphone settings");
        click(&mut h, "Accessibility settings");
        for kind in ["microphone", "accessibility"] {
            assert!(h
                .state()
                .actions
                .iter()
                .any(|a| matches!(a, Action::Permissions(k) if *k == kind)));
        }
    }
}
#[test]
fn keyboard_tab_reaches_shortcut_field() {
    let mut h = harness(fixture(), [820.0, 660.0]);
    h.get_by_role_and_label(Role::TextInput, "Keyboard shortcut")
        .focus();
    h.run();
    assert!(h
        .get_by_role_and_label(Role::TextInput, "Keyboard shortcut")
        .is_focused());
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Apply shortcut").is_focused());
}
#[test]
#[ignore = "renders review PNGs using the local GPU; run explicitly"]
fn render_all_pages_for_visual_review() {
    std::fs::create_dir_all("target/ui-review").unwrap();
    for (width, height) in [(820, 660), (720, 520)] {
        for page in Page::ALL {
            let mut state = fixture();
            state.view.page = page;
            state.screen.models[2].downloaded = true;
            state
                .screen
                .settings
                .set_quiet("model", "canary-180m-flash".into());
            let mut h = harness(state, [width as f32, height as f32]);
            h.run();
            h.render()
                .unwrap()
                .save(format!("target/ui-review/{page:?}-{width}.png"))
                .unwrap();
        }
    }
    for (name, phase) in [("Recording", "listening"), ("Processing", "working")] {
        let mut state = fixture();
        state.screen.status.phase = phase;
        state.screen.status.level = 0.12;
        state.screen.models[1].downloaded = true;
        let mut h = harness(state, [820.0, 660.0]);
        h.run_steps(3);
        h.render()
            .unwrap()
            .save(format!("target/ui-review/{name}.png"))
            .unwrap();
    }
}

#[test]
fn standard_window_shortcuts_and_escape_recording_cancel() {
    let mut state = fixture();
    state.view.page = Page::About;
    let mut h = harness(state, [820.0, 660.0]);
    h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::Comma]);
    h.run();
    assert_eq!(h.state().view.page, Page::General);
    h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::W]);
    h.run();
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Hide)));
    h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::Q]);
    h.run();
    assert!(h.state().actions.iter().any(|a| matches!(a, Action::Quit)));
    h.state_mut().screen.status.phase = "listening";
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h
        .state()
        .actions
        .iter()
        .any(|a| matches!(a, Action::Command("cancel"))));
}

#[test]
fn missing_microphone_and_deleted_model_show_recovery_instructions() {
    let mut state = fixture();
    state.view.page = Page::Audio;
    state.screen.microphones.clear();
    let mut h = harness(state, [720.0, 520.0]);
    h.get_by_label("Refresh devices");
    assert!(h
        .get_by_label("Record a sample")
        .accesskit_node()
        .is_disabled());
    click(&mut h, "General");
    click(&mut h, "Choose a speech model");
    assert_eq!(h.state().view.page, Page::Models);
}

#[test]
fn recording_controls_fit_the_minimum_window() {
    let mut state = fixture();
    state.screen.status.phase = "listening";
    let h = harness(state, [720.0, 520.0]);
    for label in ["Finish recording", "Cancel recording"] {
        let rect = h.get_by_label(label).rect();
        assert!(
            rect.min.x >= 184.0 && rect.max.x <= 720.0 && rect.max.y < 520.0,
            "{label} is clipped: {rect:?}"
        );
    }
}
