//! Development helper: transcribe a WAV file with a downloaded model.
//!
//! ```sh
//! cargo run --release --example transcribe -- parakeet-tdt-0.6b-v3 samples/jfk.wav en 5
//! cargo run --example transcribe -- parakeet-tdt-0.6b-v3 samples/jfk.wav
//! cargo run --example transcribe -- canary-180m-flash samples/jfk.wav fr
//! ```
//!
//! The optional third argument sets the spoken language for Canary; the fourth
//! repeats inference to measure both first-use and warm latency.

use anyhow::{Context, Result};
use bolum_lib::engines::Loaded;
use bolum_lib::models;
use bolum_lib::settings::SettingsStore;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let model_id = args.next().unwrap_or_else(|| "parakeet-tdt-0.6b-v3".into());
    let wav = args
        .next()
        .context("usage: transcribe <model-id> <wav-file> [language]")?;
    let language = args.next();
    let repeats = args
        .next()
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(1)
        .clamp(1, 100);

    let model = models::find(&model_id)?;
    if !models::is_downloaded(model) {
        anyhow::bail!(
            "model '{model_id}' is not downloaded; expected files in {}",
            models::model_dir(&model_id).display()
        );
    }

    let mut reader = hound::WavReader::open(&wav).with_context(|| format!("open {wav}"))?;
    let spec = reader.spec();
    let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|sample| sample as f32 / i16::MAX as f32)
            .collect(),
        (hound::SampleFormat::Int, _) => reader
            .samples::<i32>()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|sample| sample as f32 / i32::MAX as f32)
            .collect(),
        (hound::SampleFormat::Float, _) => {
            reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?
        }
    };
    let mono: Vec<f32> = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect()
    } else {
        samples
    };
    let samples = bolum_lib::audio::resample(&mono, spec.sample_rate, 16_000);
    println!(
        "{} · {:.1}s of audio · {}",
        model.name,
        samples.len() as f32 / 16_000.0,
        models::model_dir(&model_id).display()
    );

    let mut settings = SettingsStore::load();
    if let Some(language) = language {
        settings.set_quiet("language", language.into());
    }

    let started = std::time::Instant::now();
    let mut engine = Loaded::load(model)?;
    println!("Model load: {} ms", started.elapsed().as_millis());
    for iteration in 1..=repeats {
        let started = std::time::Instant::now();
        let text = engine.transcribe(&samples, &settings)?;
        println!(
            "Run {iteration}: [{} ms] {text}",
            started.elapsed().as_millis()
        );
    }
    Ok(())
}
