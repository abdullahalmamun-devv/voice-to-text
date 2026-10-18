use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub model_id: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub source: String,
    pub license: String,
    pub runtime: String,
    pub evaluated: bool,
}
pub fn inspect(path: &Path) -> Result<u64, String> {
    if crate::model_pack::is_pack(path) {
        return crate::model_pack::inspect(path);
    }
    let mut f = File::open(path).map_err(|_| "Model file could not be opened")?;
    let n = f.metadata().map_err(|e| e.to_string())?.len();
    if !(1_000_000..=4_000_000_000).contains(&n) {
        return Err("Unsupported model size".into());
    }
    let mut magic = [0; 4];
    f.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if u32::from_le_bytes(magic) != 0x67676d6c {
        return Err("Expected Whisper GGML model (.bin)".into());
    }
    let mut dims = [0u8; 44];
    f.read_exact(&mut dims).map_err(|e| e.to_string())?;
    let vocab = i32::from_le_bytes(dims[0..4].try_into().map_err(|_| "Header")?);
    if vocab < 51865 {
        return Err("English-only model is not supported for Bengali dictation".into());
    }
    f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    Ok(n)
}
pub fn hash(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut sha = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = f.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        sha.update(&b[..n]);
    }
    Ok(format!("{:x}", sha.finalize()))
}
pub fn verify(path: &Path) -> Result<(), String> {
    if crate::model_pack::is_pack(path) {
        crate::model_pack::verify(path)?;
        return Ok(());
    }
    inspect(path)?;
    let manifest = path.with_extension("manifest.json");
    if manifest.exists() {
        let m: Manifest =
            serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if m.size_bytes != std::fs::metadata(path).map_err(|e| e.to_string())?.len()
            || m.sha256 != hash(path)?
        {
            return Err("Model checksum mismatch. Reinstall the local pack.".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_and_script_models_rejected() {
        let p = std::env::temp_dir().join(format!("vtt-model-{}.bin", std::process::id()));
        std::fs::write(&p, vec![0u8; 1_000_000]).unwrap();
        assert!(inspect(&p).is_err());
        std::fs::write(&p, b"import os").unwrap();
        assert!(inspect(&p).is_err());
        std::fs::remove_file(p).unwrap();
    }
    #[test]
    fn sha_matches_known_vector() {
        let p = std::env::temp_dir().join(format!("vtt-hash-{}", std::process::id()));
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(
            hash(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        std::fs::remove_file(p).unwrap();
    }
}
