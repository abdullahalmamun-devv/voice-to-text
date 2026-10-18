//! Explicit local decoder diagnostic. Emits text only when invoked by its operator.
use std::fmt::Write;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() == 3, "MODEL AUDIO.wav");
    let samples = voicetotext::audio::read_wav(Path::new(&args[2])).unwrap();
    let context =
        WhisperContext::new_with_params(&args[1], WhisperContextParameters::default()).unwrap();
    let mut state = context.create_state().unwrap();
    let mut params = FullParams::new(SamplingStrategy::BeamSearch {
        beam_size: 5,
        patience: -1.,
    });
    params.set_n_threads(6);
    params.set_language(Some("bn"));
    params.set_translate(false);
    params.set_no_context(true);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_suppress_nst(true);
    params.set_temperature(0.);
    params.set_temperature_inc(0.2);
    state.full(params, &samples).unwrap();
    let mut bytes = Vec::new();
    let mut segments = Vec::new();
    for segment in state.as_iter() {
        let raw = segment.to_bytes().unwrap();
        bytes.extend_from_slice(raw);
        let mut raw_hex = String::with_capacity(raw.len() * 2);
        for byte in raw {
            write!(&mut raw_hex, "{byte:02x}").unwrap();
        }
        segments.push(serde_json::json!({
            "raw_hex": raw_hex,
            "text": std::str::from_utf8(raw).ok(),
            "no_speech_probability": segment.no_speech_probability(),
            "begin": segment.start_timestamp(), "end": segment.end_timestamp(),
        }));
    }
    println!(
        "{}",
        serde_json::json!({"segments":segments,"concatenated_utf8_valid":std::str::from_utf8(&bytes).is_ok(),"concatenated_text":std::str::from_utf8(&bytes).ok()})
    );
}
