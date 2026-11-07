//! Engine dispatch: one cached model is kept resident at a time.

pub mod onnx;

use anyhow::Result;

use crate::models::{CatalogModel, Engine};
use crate::settings::SettingsStore;

/// Load while the microphone is recording, without blocking Stop or Cancel.
/// One engine is retained between dictations; switching models releases it first.
#[derive(Default)]
pub struct Cache {
    id: String,
    loaded: Option<Loaded>,
    loading: Option<std::thread::JoinHandle<Result<Loaded>>>,
}
impl Cache {
    pub fn prepare(&mut self, model: &'static CatalogModel) {
        if self.id == model.id && (self.loaded.is_some() || self.loading.is_some()) {
            return;
        }
        self.loaded = None;
        if let Some(previous) = self.loading.take() {
            // Only relevant when switching immediately after cancelling a cold start.
            drop(previous.join());
        }
        self.id = model.id.into();
        self.loading = Some(std::thread::spawn(move || Loaded::load(model)));
    }

    pub fn get(&mut self) -> Result<&mut Loaded> {
        if let Some(task) = self.loading.take() {
            self.loaded = Some(
                task.join()
                    .map_err(|_| anyhow::anyhow!("Model loader stopped unexpectedly"))??,
            );
        }
        self.loaded
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Model is not loaded"))
    }
}

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
