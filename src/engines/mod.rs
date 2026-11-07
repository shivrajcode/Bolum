//! Engine dispatch: one cached model is kept resident at a time.

pub mod onnx;

use anyhow::Result;

use crate::models::{CatalogModel, Engine};
use crate::settings::SettingsStore;

pub enum Loaded {
    Parakeet(onnx::ParakeetEngine),
    Canary(onnx::CanaryEngine),
}

impl Loaded {
    pub fn load(model: &CatalogModel) -> Result<Self> {
        let dir = crate::models::model_dir(model.id);
        match model.engine {
            Engine::Parakeet => Ok(Self::Parakeet(onnx::ParakeetEngine::load(&dir)?)),
            Engine::Canary => Ok(Self::Canary(onnx::CanaryEngine::load(&dir)?)),
        }
    }

    pub fn transcribe(&mut self, samples: &[f32], settings: &SettingsStore) -> Result<String> {
        let language = settings.str("language");
        match self {
            Self::Parakeet(engine) => engine.transcribe(samples),
            Self::Canary(engine) => engine.transcribe(
                samples,
                &language,
                if settings.bool("translate_english") {
                    "en"
                } else {
                    ""
                },
            ),
        }
    }
}
