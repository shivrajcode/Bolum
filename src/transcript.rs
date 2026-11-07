//! Small, bounded session history and deterministic dictation corrections.
use crate::settings::SettingsStore;
#[derive(Clone)]
pub struct Entry {
    pub id: u64,
    pub text: String,
    pub model: String,
}

pub fn clean(text: &str, settings: &SettingsStore) -> String {
    let mut words: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
    if settings.bool("remove_fillers") {
        words.retain(|word| {
            !matches!(
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
                    .as_str(),
                "um" | "uh" | "umm" | "uhh"
            )
        });
    }
    let mut result = if settings.bool("remove_fillers") {
        words.join(" ")
    } else {
        text.to_owned()
    };
    // Exact whole-word/phrase corrections; this does not bias the speech model.
    for line in settings.str("word_replacements").lines().take(50) {
        let Some((spoken, written)) = line.split_once('=') else {
            continue;
        };
        let spoken = spoken.trim();
        let written = written.trim();
        if spoken.is_empty() || written.is_empty() {
            continue;
        }
        let mut output = String::new();
        let mut position = 0;
        for (index, _) in result.match_indices(spoken) {
            let end = index + spoken.len();
            let boundary = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric() && c != '_');
            if index < position
                || !boundary(result[..index].chars().next_back())
                || !boundary(result[end..].chars().next())
            {
                continue;
            }
            output.push_str(&result[position..index]);
            output.push_str(written);
            position = end;
        }
        output.push_str(&result[position..]);
        result = output;
    }
    result
}
pub fn retain(entries: &mut Vec<Entry>, limit: usize) {
    entries.truncate(limit.min(100));
}
/// Energy-based quiet-edge trimming, with 160 ms of context retained.
pub fn speech_slice(samples: &[f32]) -> &[f32] {
    let active = |chunk: &[f32]| {
        chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len().max(1) as f32 > 0.000_036
    };
    let Some(first) = samples.chunks(320).position(active) else {
        return &[];
    };
    let last = samples
        .chunks(320)
        .enumerate()
        .filter(|(_, chunk)| active(chunk))
        .map(|(i, _)| i)
        .next_back()
        .unwrap();
    &samples[(first * 320).saturating_sub(2560)..((last + 1) * 320 + 2560).min(samples.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_cleanup_preserves_original_whitespace() {
        assert_eq!(
            clean("First line.\nSecond line.", &SettingsStore::load_defaults()),
            "First line.\nSecond line."
        );
    }
    #[test]
    fn corrections_respect_word_boundaries_and_leave_other_words_intact() {
        let mut settings = SettingsStore::load_defaults();
        settings.set_quiet("remove_fillers", true.into());
        settings.set_quiet("word_replacements", "bolam=Bolum\nart=Art".into());
        assert_eq!(
            clean("Um, bolam is an art app, not a cart.", &settings),
            "Bolum is an Art app, not a cart."
        );
    }
    #[test]
    fn silence_detection_keeps_context_and_rejects_silence() {
        assert!(speech_slice(&vec![0.0; 16_000]).is_empty());
        let mut samples = vec![0.0; 32_000];
        samples[16_000..16_640].fill(0.1);
        let speech = speech_slice(&samples);
        assert!(speech.len() < samples.len());
        assert!(speech.contains(&0.1));
        assert!(speech.len() >= 640 + 2 * 2560);
    }
    #[test]
    fn history_is_bounded_and_zero_disables_it() {
        let mut entries: Vec<_> = (0..150)
            .map(|id| Entry {
                id,
                text: "words".into(),
                model: "model".into(),
            })
            .collect();
        retain(&mut entries, 1000);
        assert_eq!(entries.len(), 100);
        retain(&mut entries, 0);
        assert!(entries.is_empty());
    }
}
