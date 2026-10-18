//! Exact app-owned insertion readback; only control line endings may differ.
pub fn inserted_count(before: &str, after: &str, caret: u32, text: &str) -> Option<u32> {
    let old: Vec<u16> = before.encode_utf16().collect();
    let at = caret as usize;
    if at > old.len()
        || (at > 0
            && at < old.len()
            && (0xd800..=0xdbff).contains(&old[at - 1])
            && (0xdc00..=0xdfff).contains(&old[at]))
    {
        return None;
    }
    let lf = text.replace("\r\n", "\n").replace('\r', "\n");
    for variant in [text.to_string(), lf.clone(), lf.replace('\n', "\r\n")] {
        let units: Vec<u16> = variant.encode_utf16().collect();
        let mut expected = old.clone();
        expected.splice(at..at, units.iter().copied());
        if expected == after.encode_utf16().collect::<Vec<_>>() {
            return Some(units.len() as u32);
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_unicode_and_control_line_endings() {
        assert_eq!(inserted_count("ab", "aবাংলা\r\n🙂b", 1, "বাংলা\n🙂"), Some(9));
        assert_eq!(inserted_count("ab", "aবাংলা\n🙂b", 1, "বাংলা\r\n🙂"), Some(8));
    }
    #[test]
    fn unrelated_edits_and_surrogate_split_rejected() {
        assert_eq!(inserted_count("ab", "aXc", 1, "X"), None);
        assert_eq!(inserted_count("🙂", "x🙂", 1, "x"), None);
        assert_eq!(inserted_count("ab", "abX", 8, "X"), None);
    }
}
