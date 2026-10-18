//! The experimental profile accepts only pinned model and native library bytes.
//! A selected model descriptor cannot authorize loading an arbitrary shared library.
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    schema: u32,
    engine: String,
    runtime_version: String,
    platform: String,
    experimental: bool,
}
#[derive(Deserialize)]
pub(crate) struct FilePin {
    pub name: String,
    pub size_bytes: u64,
    pub sha256: String,
}
#[derive(Deserialize)]
pub(crate) struct RuntimePins {
    pub files: Vec<FilePin>,
    pub capi: String,
    pub onnx: String,
}
#[derive(Deserialize)]
struct Pins {
    engine: String,
    runtime_version: String,
    model_files: Vec<FilePin>,
    runtimes: BTreeMap<String, RuntimePins>,
}
pub(crate) struct Pack {
    pub root: PathBuf,
    pub runtime: RuntimePins,
    pub size_bytes: u64,
}
pub fn is_pack(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".vtt-model.json"))
}
fn platform() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok("windows-x64"),
        ("linux", "x86_64") => Ok("linux-x64"),
        _ => Err("Experimental Bengali model supports Windows/Linux x64 only".into()),
    }
}
fn descriptor(bytes: &[u8], expected_platform: &str, pins: &Pins) -> Result<(), String> {
    let value: Descriptor =
        serde_json::from_slice(bytes).map_err(|_| "Invalid Bengali model profile")?;
    if value.schema != 1
        || value.engine != pins.engine
        || value.runtime_version != pins.runtime_version
        || value.platform != expected_platform
        || !value.experimental
    {
        return Err("Unsupported or mismatched experimental Bengali profile".into());
    }
    Ok(())
}
fn checked_file(root: &Path, pin: &FilePin, hash: bool) -> Result<(), String> {
    // Names come from compiled pins, never from the user-selected JSON.
    let path = root.join(&pin.name);
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|_| format!("Missing local pack file: {}", pin.name))?;
    if !metadata.file_type().is_file() || metadata.len() != pin.size_bytes {
        return Err(format!("Invalid local pack file: {}", pin.name));
    }
    if hash && crate::model::hash(&path)? != pin.sha256 {
        return Err(format!("Local pack checksum mismatch: {}", pin.name));
    }
    Ok(())
}
fn check(path: &Path, hash: bool) -> Result<Pack, String> {
    if !is_pack(path)
        || std::fs::metadata(path)
            .map_err(|_| "Model profile missing")?
            .len()
            > 16384
    {
        return Err("Invalid Bengali model profile".into());
    }
    let mut pins: Pins =
        serde_json::from_str(include_str!("../native/bengali-zipformer-pins.json"))
            .map_err(|_| "Invalid compiled model pins")?;
    let platform = platform()?;
    descriptor(
        &std::fs::read(path).map_err(|e| e.to_string())?,
        platform,
        &pins,
    )?;
    let root = path
        .parent()
        .ok_or("Model folder unavailable")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let runtime = pins
        .runtimes
        .remove(platform)
        .ok_or("Runtime unavailable")?;
    let runtime_dir = root.join("runtime");
    if std::fs::symlink_metadata(&runtime_dir)
        .map_err(|_| "Local runtime folder missing")?
        .file_type()
        .is_symlink()
    {
        return Err("Runtime folder must not be a symlink".into());
    }
    for pin in &pins.model_files {
        checked_file(&root, pin, hash)?;
    }
    for pin in &runtime.files {
        checked_file(&runtime_dir, pin, hash)?;
    }
    Ok(Pack {
        root,
        runtime,
        size_bytes: pins.model_files.iter().map(|p| p.size_bytes).sum(),
    })
}
pub fn inspect(path: &Path) -> Result<u64, String> {
    Ok(check(path, false)?.size_bytes)
}
pub(crate) fn verify(path: &Path) -> Result<Pack, String> {
    check(path, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_cannot_change_runtime_or_claim_qualification() {
        let pins: Pins =
            serde_json::from_str(include_str!("../native/bengali-zipformer-pins.json")).unwrap();
        let mut value = serde_json::json!({"schema":1,"engine":"bengali-zipformer","runtime_version":"1.13.8","platform":"windows-x64","experimental":true});
        assert!(descriptor(&serde_json::to_vec(&value).unwrap(), "windows-x64", &pins).is_ok());
        assert!(descriptor(&serde_json::to_vec(&value).unwrap(), "linux-x64", &pins).is_err());
        value["library"] = "evil.dll".into();
        assert!(descriptor(&serde_json::to_vec(&value).unwrap(), "windows-x64", &pins).is_err());
        value.as_object_mut().unwrap().remove("library");
        value["experimental"] = false.into();
        assert!(descriptor(&serde_json::to_vec(&value).unwrap(), "windows-x64", &pins).is_err());
    }
    #[test]
    fn pinned_library_must_match_before_loading() {
        let root = std::env::temp_dir().join(format!("vtt-pack-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("runtime.dll"), b"bad").unwrap();
        let pin = FilePin {
            name: "runtime.dll".into(),
            size_bytes: 3,
            sha256: "0".repeat(64),
        };
        assert!(checked_file(&root, &pin, false).is_ok());
        assert!(checked_file(&root, &pin, true).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
