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
    if samples.len() as u64 / channels as u64 > rate as u64 * 600 {
        return Err("Local WAV files are limited to 10 minutes".into());
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
    if rate == SAMPLE_RATE as u32 {
        return Ok(mono);
    }
    let step = rate as f64 / SAMPLE_RATE as f64;
    let cutoff = (SAMPLE_RATE as f64 / rate as f64).min(1.) * 0.94;
    let radius = (16. / cutoff).ceil() as isize;
    Ok((0..output_len as usize)
        .map(|i| {
            let position = i as f64 * step;
            let center = position.floor() as isize;
            let (mut value, mut weights) = (0., 0.);
            for index in (center - radius)..=(center + radius) {
                if index < 0 || index >= mono.len() as isize {
                    continue;
                }
                let distance = index as f64 - position;
                if distance.abs() > radius as f64 {
                    continue;
                }
                let x = std::f64::consts::PI * cutoff * distance;
                let sinc = if x.abs() < 1e-10 { 1. } else { x.sin() / x };
                let weight = sinc
                    * cutoff
                    * (0.5 + 0.5 * (std::f64::consts::PI * distance / radius as f64).cos());
                value += mono[index as usize] as f64 * weight;
                weights += weight;
            }
            if weights.abs() > 1e-10 {
                (value / weights).clamp(-1., 1.) as f32
            } else {
                0.
            }
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
    if !(8000..=192000).contains(&spec.sample_rate) || !(1..=8).contains(&spec.channels) {
        return Err("Unsupported WAV layout".into());
    }
    if reader.duration() as u64 > spec.sample_rate as u64 * 600 {
        return Err("Local WAV files are limited to 10 minutes".into());
    }
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

#[cfg(test)]
mod alias_tests {
    use super::*;
    #[test]
    fn out_of_band_tone_does_not_alias_into_speech() {
        let tone = |hz: f32| {
            (0..48000)
                .map(|i| (2. * std::f32::consts::PI * hz * i as f32 / 48000.).sin() * 0.5)
                .collect::<Vec<_>>()
        };
        let rms = |v: Vec<f32>| (v[500..15500].iter().map(|x| x * x).sum::<f32>() / 15000.).sqrt();
        assert!(rms(resample_mono(&tone(1000.), 48000, 1).unwrap()) > 0.3);
        assert!(rms(resample_mono(&tone(12000.), 48000, 1).unwrap()) < 0.02);
    }
}
