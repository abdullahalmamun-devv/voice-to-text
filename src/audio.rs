use std::path::Path;
pub const SAMPLE_RATE: usize = 16000;
pub const MAX_TURN: usize = SAMPLE_RATE * 120;
pub const MAX_FILE: usize = SAMPLE_RATE * 600;
pub fn speech_energy(samples: &[f32]) -> bool {
    // A conservative signal gate, not a trained VAD or recognition confidence.
    if samples.len() < 1600 {
        return false;
    }
    samples
        .chunks(1600)
        .filter(|chunk| {
            let rms = (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
            rms >= 0.003
        })
        .take(2)
        .count()
        >= 2
}
pub fn resample_mono(samples: &[f32], rate: u32, channels: u16) -> Result<Vec<f32>, String> {
    if !(8000..=192000).contains(&rate)
        || channels == 0
        || channels > 8
        || samples.len() % channels as usize != 0
    {
        return Err("Unsupported WAV layout".into());
    }
    if samples.iter().any(|x| !x.is_finite()) {
        return Err("Invalid audio samples".into());
    }
    let mono: Vec<f32> = samples
        .chunks_exact(channels as usize)
        .map(|c| c.iter().sum::<f32>() / channels as f32)
        .collect();
    let output_len = mono.len() as u64 * SAMPLE_RATE as u64 / rate as u64;
    if output_len > MAX_FILE as u64 {
        return Err("Local WAV files are limited to 10 minutes".into());
    }
    if mono.is_empty() {
        return Err("Audio file is empty".into());
    }
    let step = rate as f64 / SAMPLE_RATE as f64;
    Ok((0..output_len as usize)
        .map(|i| {
            let t = i as f64 * step;
            let lo = t as usize;
            let hi = (lo + 1).min(mono.len() - 1);
            let f = (t - lo as f64) as f32;
            (mono[lo] * (1. - f) + mono[hi] * f).clamp(-1., 1.)
        })
        .collect())
}
pub fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 250_000_000 {
        return Err("WAV file exceeds 250 MB".into());
    }
    let reader =
        hound::WavReader::open(path).map_err(|_| "Only uncompressed PCM/float WAV is supported")?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .map(|x| x.map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            if ![8, 16, 24, 32].contains(&spec.bits_per_sample) {
                return Err("Unsupported PCM depth".into());
            }
            let scale = 2f32.powi(spec.bits_per_sample as i32 - 1);
            reader
                .into_samples::<i32>()
                .map(|x| x.map(|v| v as f32 / scale).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?
        }
    };
    resample_mono(&samples, spec.sample_rate, spec.channels)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_and_short_click_rejected() {
        assert!(!speech_energy(&vec![0.; 32000]));
        let mut click = vec![0.; 32000];
        click[10] = 1.;
        assert!(!speech_energy(&click));
    }
    #[test]
    fn quiet_speech_retained() {
        assert!(speech_energy(&vec![0.004; 16000]));
    }
    #[test]
    fn stereo_resampling_duration_and_mix() {
        let v = resample_mono(&vec![0.5; 48000 * 2], 48000, 2).unwrap();
        assert_eq!(v.len(), 16000);
        assert!((v[100] - 0.5).abs() < 0.0001);
    }
    #[test]
    fn malformed_audio_rejected() {
        assert!(resample_mono(&[f32::NAN], 16000, 1).is_err());
        assert!(resample_mono(&[0.1], 16000, 0).is_err());
        assert!(resample_mono(&[0.1], 16000, 2).is_err());
    }
}
