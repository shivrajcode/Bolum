//! Small JSON settings store. Existing NVIDIA model downloads remain reusable.
use serde_json::{json, Map, Value};
use std::path::PathBuf;

fn base_dir(xdg: &str, windows: &str, fallback: &str) -> PathBuf {
    std::env::var_os(xdg).map(PathBuf::from).unwrap_or_else(|| {
        if cfg!(windows) {
            if let Some(path) = std::env::var_os(windows) {
                return path.into();
            }
        }
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(fallback)
    })
}
pub fn config_dir() -> PathBuf {
    base_dir("XDG_CONFIG_HOME", "APPDATA", ".config").join("bolum")
}
pub fn data_dir() -> PathBuf {
    base_dir("XDG_DATA_HOME", "LOCALAPPDATA", ".local/share").join("bolum")
}
pub fn models_dir() -> PathBuf {
    data_dir().join("models")
}
pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}
pub fn runtime_dir() -> PathBuf {
    config_dir()
}
pub fn defaults() -> Value {
    json!({
        "hotkey": "ctrl+alt+space", "push_to_talk": false, "microphone": null,
        "model": "parakeet-tdt-0.6b-v3", "language": "en",
        "injection": if cfg!(any(target_os="macos", windows)) { "clipboard" } else { "type" },
        "typing_tool": "auto", "append_trailing_space": true
    })
}
#[derive(Clone)]
pub struct SettingsStore {
    value: Map<String, Value>,
}
impl SettingsStore {
    #[cfg(test)]
    pub fn load_defaults() -> Self {
        Self::from_value(defaults())
    }

    pub fn load() -> Self {
        let stored = std::fs::read(settings_path())
            .ok()
            .and_then(|raw| serde_json::from_slice::<Value>(&raw).ok());
        Self::from_value(stored.unwrap_or(Value::Null))
    }
    fn from_value(stored: Value) -> Self {
        let mut value = defaults().as_object().unwrap().clone();
        for (key, default) in &mut value {
            if let Some(saved) = stored.get(key) {
                if saved.is_null()
                    || std::mem::discriminant(saved) == std::mem::discriminant(default)
                {
                    *default = saved.clone();
                }
            }
        }
        if crate::models::find(value["model"].as_str().unwrap_or_default()).is_err() {
            value.insert("model".into(), json!("parakeet-tdt-0.6b-v3"));
        }
        Self { value }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(config_dir())?;
        let temp = config_dir().join("settings.json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(&self.value)?)?;
        std::fs::rename(temp, settings_path())?;
        Ok(())
    }
    pub fn set_quiet(&mut self, key: &str, value: Value) {
        self.value.insert(key.into(), value);
    }
    pub fn str(&self, key: &str) -> String {
        self.value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into()
    }
    pub fn bool(&self, key: &str) -> bool {
        self.value
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
    pub fn opt_str(&self, key: &str) -> Option<String> {
        let value = self.str(key);
        (!value.is_empty()).then_some(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrates_removed_models_and_discards_cloud_credentials() {
        let store = SettingsStore::from_value(
            json!({"model":"whisper-base.en", "post_process":{"api_key":"secret"}, "hotkey":"ctrl+shift+d"}),
        );
        assert_eq!(store.str("model"), "parakeet-tdt-0.6b-v3");
        assert_eq!(store.str("hotkey"), "ctrl+shift+d");
        assert!(!store.value.contains_key("post_process"));
    }
}
