//! Explicit opt-in: downloads and verifies real weights, then transcribes speech.
use bolum_lib::{engines::Cache, models, settings::SettingsStore};
use std::sync::{atomic::AtomicBool, Arc};

#[tokio::test]
#[ignore = "downloads up to 884 MB of NVIDIA weights; run with isolated XDG_DATA_HOME"]
async fn nvidia_engines_transcribe_speech() -> anyhow::Result<()> {
    let mut reader = hound::WavReader::open("tests/fixtures/quick-brown-fox.wav")?;
    let spec = reader.spec();
    let raw = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;
    let mono: Vec<f32> = raw
        .chunks(spec.channels as usize)
        .map(|frame| frame.iter().map(|s| *s as f32 / 32768.0).sum::<f32>() / frame.len() as f32)
        .collect();
    let samples = bolum_lib::audio::resample(&mono, spec.sample_rate, 16_000);
    let mut cache = Cache::default();
    for id in ["canary-180m-flash", "parakeet-tdt-0.6b-v3"] {
        let model = models::find(id)?;
        models::download(model, Arc::new(AtomicBool::new(false)), |_, _| {}).await?;
        cache.prepare(model);
        let text = cache.get()?.transcribe(&samples, &SettingsStore::load())?;
        println!("{id}: {text}");
        let normalized = text.to_lowercase();
        assert!(
            normalized.contains("quick brown fox"),
            "unexpected {id} transcript: {text}"
        );
        // Repeated dictations reuse the prepared engine, including after a model switch.
        let engine = cache.get()? as *const _;
        cache.prepare(model);
        assert_eq!(engine, cache.get()? as *const _);
        assert_eq!(
            text,
            cache.get()?.transcribe(&samples, &SettingsStore::load())?
        );
    }
    Ok(())
}
