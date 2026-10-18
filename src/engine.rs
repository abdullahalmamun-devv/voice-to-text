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
}
pub struct Engine {
    context: WhisperContext,
    pub load_ms: u64,
}
impl Engine {
    pub fn load(path: &Path) -> Result<Self, String> {
        model::verify(path)?;
        let start = Instant::now();
        #[cfg(not(all(windows, target_env = "gnu")))]
        let loaded = WhisperContext::new_with_params(
            path.to_str().ok_or("Model path is not valid UTF-8")?,
            WhisperContextParameters::default(),
        );
        // Upstream's GNU Windows file loader uses narrow paths. Buffer loading
        // preserves Bengali profile/package paths without changing the decoder.
        #[cfg(all(windows, target_env = "gnu"))]
        let loaded = {
            use std::io::Read;
            let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let size = file.metadata().map_err(|e| e.to_string())?.len() as usize;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(size)
                .map_err(|_| "Not enough memory to read the local model")?;
            file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            WhisperContext::new_from_buffer_with_params(&bytes, WhisperContextParameters::default())
        };
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
        params.set_no_context(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_temperature(0.);
        params.set_temperature_inc(0.);
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
        Ok(Transcript {
            text,
            segments,
            decode_ms: start.elapsed().as_millis() as u64,
            load_ms: self.load_ms,
        })
    }
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
