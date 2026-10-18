use crate::text::{validate_entries, DigitStyle, Entry, Snippet};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub language: String,
    pub digits: DigitStyle,
    pub theme: String,
    pub review: bool,
    pub hold_to_talk: bool,
    pub commands: bool,
    pub live_preview: bool,
    pub model_path: String,
    pub microphone: u32,
    pub hotkey: u32,
    pub modifiers: u32,
    pub saver_seconds: u64,
    pub history_enabled: bool,
    pub retention_days: u32,
    pub entries: Vec<Entry>,
    pub snippets: Vec<Snippet>,
    pub startup: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            language: "bn".into(),
            digits: DigitStyle::Preserve,
            theme: "system".into(),
            review: true,
            hold_to_talk: false,
            commands: false,
            live_preview: false,
            model_path: String::new(),
            microphone: u32::MAX,
            hotkey: 0x20,
            modifiers: 3,
            saver_seconds: 300,
            history_enabled: false,
            retention_days: 7,
            entries: Vec::new(),
            snippets: Vec::new(),
            startup: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported settings version".into());
        }
        if !["bn", "en", "auto"].contains(&self.language.as_str()) {
            return Err("Unsupported language preset".into());
        }
        if !["light", "dark", "system"].contains(&self.theme.as_str()) {
            return Err("Unsupported theme".into());
        }
        if ![0, 7, 30].contains(&self.retention_days) {
            return Err("Unsupported retention period".into());
        }
        if self.hotkey < 0x20 || self.hotkey > 0x7b || self.modifiers == 0 || self.modifiers > 15 {
            return Err("Invalid shortcut".into());
        }
        if self.saver_seconds != 0 && !(30..=3600).contains(&self.saver_seconds) {
            return Err("Invalid saver timeout".into());
        }
        validate_entries(&self.entries)?;
        if self.snippets.len() > 100 {
            return Err("At most 100 snippets are supported".into());
        }
        let mut keys = std::collections::HashSet::new();
        for s in &self.snippets {
            if s.trigger.trim().is_empty()
                || s.trigger.chars().count() > 80
                || s.text.is_empty()
                || s.text.len() > 16000
                || s.text.contains('\0')
                || s.trigger.contains(['\0', '\n', '\r'])
                || !keys.insert(&s.trigger)
            {
                return Err("Invalid or duplicate snippet".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: u64,
    pub timestamp: u64,
    pub text: String,
}
#[derive(Serialize, Deserialize)]
struct History {
    schema_version: u32,
    items: Vec<HistoryItem>,
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("VoiceToText")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join(".config")
            })
            .join("voicetotext")
    }
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("No parent folder")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&temp, path).map_err(|e| e.to_string())
}
pub fn read_settings(dir: &Path) -> Result<Settings, String> {
    let p = dir.join("settings.json");
    if !p.exists() {
        return Ok(Settings::default());
    }
    let parse = |p: &Path| -> Result<Settings, String> {
        let bytes = fs::read(p).map_err(|e| e.to_string())?;
        if bytes.len() > 1_000_000 {
            return Err("Settings file too large".into());
        }
        let s: Settings = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        s.validate()?;
        Ok(s)
    };
    match parse(&p) {
        Ok(s) => Ok(s),
        Err(e) => {
            let backup = dir.join("settings.backup.json");
            Err(if parse(&backup).is_ok() {
                format!("Settings invalid ({e}). A valid settings.backup.json exists; restore it manually before reopening.")
            } else {
                format!("Settings invalid ({e}). Original file retained. Repair or rename it before reopening.")
            })
        }
    }
}
pub fn save_settings(dir: &Path, s: &Settings) -> Result<(), String> {
    s.validate()?;
    let p = dir.join("settings.json");
    if p.exists() {
        let prior = fs::read(&p).map_err(|e| e.to_string())?;
        if serde_json::from_slice::<Settings>(&prior).is_ok() {
            atomic_write(&dir.join("settings.backup.json"), &prior)?;
        }
    }
    atomic_write(
        &p,
        &serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?,
    )
}
pub fn history(dir: &Path, retention_days: u32) -> Result<Vec<HistoryItem>, String> {
    let p = dir.join("history.json");
    if !p.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&p).map_err(|e| e.to_string())?;
    if bytes.len() > 10_000_000 {
        return Err("History file exceeds limit".into());
    }
    let mut h: History = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if h.schema_version != 1 {
        return Err("Unsupported history schema".into());
    }
    let original = h.items.len();
    let cutoff = now().saturating_sub(retention_days as u64 * 86400);
    if retention_days != 0 {
        h.items.retain(|x| x.timestamp >= cutoff);
    }
    if h.items.len() != original {
        atomic_write(&p, &serde_json::to_vec(&h).map_err(|e| e.to_string())?)?;
    }
    Ok(h.items)
}
pub fn append_history(dir: &Path, s: &Settings, text: &str) -> Result<(), String> {
    if !s.history_enabled || text.is_empty() {
        return Ok(());
    }
    let mut items = history(dir, s.retention_days)?;
    let id = items.last().map(|i| i.id.saturating_add(1)).unwrap_or(1);
    items.push(HistoryItem {
        id,
        timestamp: now(),
        text: text.into(),
    });
    while items.len() > 500 {
        items.remove(0);
    }
    atomic_write(
        &dir.join("history.json"),
        &serde_json::to_vec(&History {
            schema_version: 1,
            items,
        })
        .map_err(|e| e.to_string())?,
    )
}
pub fn delete_history(dir: &Path) -> Result<(), String> {
    for p in [dir.join("history.json"), dir.join("history.tmp")] {
        if p.exists() {
            fs::remove_file(p).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn dir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("vtt-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn history_is_off_and_no_speech_file_created() {
        let p = dir("privacy");
        append_history(&p, &Settings::default(), "private speech").unwrap();
        assert!(!p.join("history.json").exists());
        assert_eq!(fs::read_dir(&p).unwrap().count(), 0);
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn settings_roundtrip_and_backup() {
        let p = dir("settings");
        let mut s = Settings::default();
        save_settings(&p, &s).unwrap();
        s.review = false;
        save_settings(&p, &s).unwrap();
        assert!(!read_settings(&p).unwrap().review);
        assert!(p.join("settings.backup.json").exists());
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn corrupt_settings_not_silently_reset() {
        let p = dir("corrupt");
        fs::write(p.join("settings.json"), "broken").unwrap();
        assert!(read_settings(&p).is_err());
        assert_eq!(
            fs::read_to_string(p.join("settings.json")).unwrap(),
            "broken"
        );
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn opt_in_history_and_delete() {
        let p = dir("optin");
        let s = Settings {
            history_enabled: true,
            ..Settings::default()
        };
        append_history(&p, &s, "আজ ভালো দিন").unwrap();
        assert_eq!(history(&p, 7).unwrap().len(), 1);
        delete_history(&p).unwrap();
        assert!(!p.join("history.json").exists());
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn invalid_schema_and_settings_rejected() {
        let mut s = Settings {
            schema_version: 2,
            ..Settings::default()
        };
        assert!(s.validate().is_err());
        s.schema_version = 1;
        s.retention_days = 100;
        assert!(s.validate().is_err());
    }
    #[test]
    fn unsafe_snippet_import_rejected() {
        let mut s = Settings::default();
        s.snippets.push(Snippet {
            trigger: "signature".into(),
            text: "prefix\0hidden suffix".into(),
        });
        assert!(s.validate().is_err());
        s.snippets[0].text = "safe".into();
        assert!(s.validate().is_ok());
    }
    #[test]
    fn expired_history_purged() {
        let p = dir("retention");
        let h = History {
            schema_version: 1,
            items: vec![HistoryItem {
                id: 1,
                timestamp: 0,
                text: "expired".into(),
            }],
        };
        atomic_write(&p.join("history.json"), &serde_json::to_vec(&h).unwrap()).unwrap();
        assert!(history(&p, 7).unwrap().is_empty());
        assert!(!fs::read_to_string(p.join("history.json"))
            .unwrap()
            .contains("expired"));
        fs::remove_dir_all(p).unwrap();
    }
}
