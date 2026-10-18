#![cfg_attr(windows, windows_subsystem = "windows")]
use std::path::Path;
use voicetotext::{audio, engine, worker};
#[cfg(windows)]
mod windows;
fn cli(args: &[String]) -> Result<(), String> {
    if args.first().map(String::as_str) == Some("--version") {
        println!("VoiceToText {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.first().map(String::as_str) == Some("--worker") {
        return worker::serve();
    }
    if args.first().map(String::as_str) == Some("--transcribe") {
        if !(4..=5).contains(&args.len()) {
            return Err("Usage: --transcribe MODEL AUDIO.wav OUTPUT.json [bn|en|auto]".into());
        }
        let samples = audio::read_wav(Path::new(&args[2]))?;
        let engine = engine::Engine::load(Path::new(&args[1]))?;
        let result = engine.transcribe(
            &samples,
            args.get(4).map(String::as_str).unwrap_or("bn"),
            "",
            false,
        )?;
        std::fs::write(
            &args[3],
            serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if !args.is_empty() && args != ["--hidden"] {
        return Err("Unknown command-line arguments".into());
    }
    #[cfg(windows)]
    {
        windows::run()
    }
    #[cfg(not(windows))]
    {
        Err("Desktop UI is currently Windows-only. Use --transcribe MODEL AUDIO.wav OUTPUT.json [bn|en|auto] for the local CLI.".into())
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Err(e) = cli(&args) {
        #[cfg(windows)]
        if args.is_empty() || args == ["--hidden"] {
            windows::error_box(&e);
        } else {
            eprintln!("{e}");
        }
        #[cfg(not(windows))]
        eprintln!("{e}");
        std::process::exit(1);
    }
}
