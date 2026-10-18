use crate::{
    audio::{speech_energy, MAX_FILE},
    model,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Transcript {
    pub text: String,
    pub segments: Vec<Segment>,
    pub decode_ms: u64,
    pub load_ms: u64,
    #[serde(default)]
    pub recognized_language: Option<String>,
    #[serde(default)]
    pub review_required: bool,
    #[serde(default)]
    pub timing_reliable: bool,
    #[serde(default)]
    pub no_timestamps: bool,
}
pub struct Engine {
    context: WhisperContext,
    pub load_ms: u64,
}
impl Engine {
    pub fn load(path: &Path) -> Result<Self, String> {
        model::verify(path)?;
        let start = Instant::now();
        let loaded = WhisperContext::new_with_params(
            path.to_str().ok_or("Model path is not valid UTF-8")?,
            WhisperContextParameters::default(),
        );
        let ctx = loaded
            .map_err(|_| "Model could not be loaded. Check model format and available memory.")?;
        Ok(Self {
            context: ctx,
            load_ms: start.elapsed().as_millis() as u64,
        })
    }
    pub fn transcribe(
        &self,
        samples: &[f32],
        language: &str,
        prompt: &str,
        preview: bool,
    ) -> Result<Transcript, String> {
        self.transcribe_profile(samples, language, prompt, preview, false)
    }
    pub fn transcribe_profile(
        &self,
        samples: &[f32],
        language: &str,
        prompt: &str,
        preview: bool,
        no_timestamps: bool,
    ) -> Result<Transcript, String> {
        if samples.len() > MAX_FILE || samples.iter().any(|x| !x.is_finite()) {
            return Err("Invalid or oversized audio".into());
        }
        if !["bn", "en", "auto"].contains(&language) {
            return Err("Unsupported language".into());
        }
        if prompt.contains('\0') || prompt.chars().count() > 600 {
            return Err("Invalid vocabulary prompt".into());
        }
        if !speech_energy(samples) {
            return Ok(Transcript::default());
        }
        let start = Instant::now();
        let mut state = self
            .context
            .create_state()
            .map_err(|_| "Could not allocate inference state")?;
        let mut params = FullParams::new(if preview {
            SamplingStrategy::Greedy { best_of: 1 }
        } else {
            SamplingStrategy::BeamSearch {
                beam_size: 5,
                patience: -1.,
            }
        });
        params.set_n_threads(
            std::thread::available_parallelism()
                .map(|n| n.get().min(6))
                .unwrap_or(4) as i32,
        );
        params.set_language(if language == "auto" {
            None
        } else {
            Some(language)
        });
        params.set_translate(false);
        params.set_no_timestamps(no_timestamps);
        params.set_no_context(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_temperature(0.);
        params.set_temperature_inc(if preview { 0. } else { 0.2 });
        if !prompt.is_empty() {
            params.set_initial_prompt(prompt);
        }
        state
            .full(params, samples)
            .map_err(|_| "Local inference failed")?;
        let mut segments = Vec::new();
        for segment in state.as_iter() {
            if segment.no_speech_probability() > 0.8 {
                continue;
            }
            let text = segment
                .to_str()
                .map_err(|_| "Model returned invalid text")?
                .trim()
                .to_string();
            if text.is_empty() {
                continue;
            }
            let begin = segment.start_timestamp().max(0) as u64 * 10;
            let end = segment.end_timestamp().max(0) as u64 * 10;
            segments.push(Segment {
                start_ms: begin,
                end_ms: end,
                text,
            });
        }
        let text = segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let recognized_language =
            whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_string);
        let review_required = crate::text::unexpected_script(&text, language);
        let timing_reliable = !no_timestamps
            && reliable_timing(&segments, samples.len() as u64 * 1000 / 16000);
        Ok(Transcript {
            no_timestamps,
            recognized_language,
            review_required,
            timing_reliable,
            text,
            segments,
            decode_ms: start.elapsed().as_millis() as u64,
            load_ms: self.load_ms,
        })
    }
}
pub fn reliable_timing(segments: &[Segment], audio_ms: u64) -> bool {
    let mut last = 0;
    !segments.is_empty()
        && segments.iter().all(|s| {
            let valid = s.start_ms >= last
                && s.end_ms > s.start_ms
                && s.end_ms <= audio_ms.saturating_add(100);
            last = s.end_ms;
            valid
        })
}
pub fn srt(segments: &[Segment]) -> Result<String, String> {
    if segments.is_empty() {
        return Err("No timed segments are available".into());
    }
    let format = |ms: u64| {
        format!(
            "{:02}:{:02}:{:02},{:03}",
            ms / 3_600_000,
            (ms / 60_000) % 60,
            (ms / 1000) % 60,
            ms % 1000
        )
    };
    let mut out = String::new();
    let mut last = 0;
    for (i, s) in segments.iter().enumerate() {
        if s.end_ms <= s.start_ms || s.start_ms < last {
            return Err("Unreliable segment timing; use TXT/Markdown export".into());
        }
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            format(s.start_ms),
            format(s.end_ms),
            s.text
        ));
        last = s.end_ms;
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srt_uses_engine_timestamps() {
        let text = srt(&[Segment {
            start_ms: 1250,
            end_ms: 2760,
            text: "বাংলা".into(),
        }])
        .unwrap();
        assert!(text.contains("00:00:01,250 --> 00:00:02,760"));
    }
    #[test]
    fn fabricated_or_overlapping_timing_not_exported() {
        assert!(srt(&[]).is_err());
        assert!(srt(&[Segment {
            start_ms: 5,
            end_ms: 2,
            text: "x".into()
        }])
        .is_err());
    }
}

#[cfg(test)]
mod timing_tests {
    use super::*;
    #[test]
    fn times_beyond_audio_are_not_exportable() {
        let segment = |end_ms| Segment {
            start_ms: 0,
            end_ms,
            text: "বাংলা".into(),
        };
        assert!(!reliable_timing(&[segment(30000)], 10000));
        assert!(reliable_timing(&[segment(10000)], 10000));
        assert!(!reliable_timing(&[segment(0)], 10000));
        assert!(!reliable_timing(&[], 10000));
    }
}
