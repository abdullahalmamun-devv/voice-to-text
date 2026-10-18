use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DigitStyle {
    #[default]
    Preserve,
    Bengali,
    Latin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub term: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default = "enabled")]
    pub enabled: bool,
}
fn enabled() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snippet {
    pub trigger: String,
    pub text: String,
}

pub fn validate_entries(entries: &[Entry]) -> Result<(), String> {
    if entries.len() > 500 {
        return Err("Dictionary supports at most 500 entries".into());
    }
    let mut aliases = std::collections::HashSet::new();
    for e in entries {
        if e.term.trim().is_empty() || e.term.chars().count() > 160 || e.alias.chars().count() > 160
        {
            return Err("Invalid dictionary entry length".into());
        }
        if e.term.contains(['\n', '\r', '\0']) || e.alias.contains(['\n', '\r', '\0']) {
            return Err("Entry must be one phrase".into());
        }
        if e.enabled && !e.alias.is_empty() && !aliases.insert(e.alias.clone()) {
            return Err("Conflicting aliases".into());
        }
    }
    Ok(())
}
fn word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '\u{0981}'..='\u{09ff}' | '\u{200c}' | '\u{200d}' | '_')
}

/// Longest exact phrase wins; output is never scanned again. No chained/cyclic rules.
pub fn replace_aliases(raw: &str, entries: &[Entry]) -> String {
    let mut rules: Vec<_> = entries
        .iter()
        .filter(|e| e.enabled && !e.alias.is_empty())
        .collect();
    rules.sort_by_key(|e| std::cmp::Reverse(e.alias.len()));
    let mut result = String::new();
    let mut offset = 0;
    while offset < raw.len() {
        let rest = &raw[offset..];
        let before = raw[..offset].chars().next_back();
        let rule = rules.iter().find(|e| {
            rest.starts_with(&e.alias)
                && !(before.is_some_and(word) && e.alias.chars().next().is_some_and(word))
                && !(rest[e.alias.len()..].chars().next().is_some_and(word)
                    && e.alias.chars().next_back().is_some_and(word))
        });
        if let Some(e) = rule {
            result.push_str(&e.term);
            offset += e.alias.len();
        } else {
            let c = rest.chars().next().expect("nonempty suffix");
            result.push(c);
            offset += c.len_utf8();
        }
    }
    result
}

pub fn prompt(entries: &[Entry]) -> String {
    let mut p = String::new();
    for e in entries.iter().filter(|e| e.enabled) {
        if p.chars().count() + e.term.chars().count() + 2 > 600 {
            break;
        }
        if !p.is_empty() {
            p.push_str(", ");
        }
        p.push_str(&e.term);
    }
    p
}

/// V2 editing is confined to app-owned text. Never issues editor shortcuts.
pub fn edit_owned(previous: &str, command: &str) -> Option<String> {
    match command.trim().trim_end_matches(['।', '.']) {
        "শেষ শব্দ মুছুন" => {
            let previous = previous.trim_end();
            let offset = previous
                .char_indices()
                .rev()
                .find(|(_, c)| c.is_whitespace())
                .map(|(i, _)| i)
                .unwrap_or(0);
            Some(previous[..offset].trim_end().to_string())
        }
        "শেষ লাইন মুছুন" => Some(
            previous
                .trim_end_matches('\n')
                .rsplit_once('\n')
                .map(|(head, _)| head.to_string())
                .unwrap_or_default(),
        ),
        _ => None,
    }
}

fn atom(s: &str) -> Option<u64> {
    Some(match s {
        "শূন্য" => 0,
        "এক" => 1,
        "দুই" => 2,
        "তিন" => 3,
        "চার" => 4,
        "পাঁচ" => 5,
        "ছয়" | "ছয়" => 6,
        "সাত" => 7,
        "আট" => 8,
        "নয়" | "নয়" => 9,
        "দশ" => 10,
        "এগারো" => 11,
        "বারো" => 12,
        "তেরো" => 13,
        "চৌদ্দ" => 14,
        "পনেরো" => 15,
        "ষোলো" => 16,
        "সতেরো" => 17,
        "আঠারো" => 18,
        "উনিশ" => 19,
        "বিশ" => 20,
        "ত্রিশ" => 30,
        "চল্লিশ" => 40,
        "পঞ্চাশ" => 50,
        "ষাট" => 60,
        "সত্তর" => 70,
        "আশি" => 80,
        "নব্বই" => 90,
        _ => return None,
    })
}

/// Conservative complete-phrase grammar: supported integer amounts only.
/// Adjacent spoken digits, ranges, relative dates, and classifier suffixes are preserved.
pub fn integer_phrase(raw: &str) -> Option<u64> {
    let tokens: Vec<_> = raw.split_whitespace().collect();
    if tokens.is_empty() || tokens.len() > 12 {
        return None;
    }
    let mut total = 0u64;
    let mut group = 0u64;
    let mut last_scale = u64::MAX;
    let mut pending = false;
    let mut has_scale = false;
    for token in tokens {
        let scale = match token {
            "কোটি" => Some(10_000_000),
            "লাখ" | "লক্ষ" => Some(100_000),
            "হাজার" => Some(1_000),
            "শত" => Some(100),
            _ => None,
        };
        if let Some(scale) = scale {
            if !pending || scale >= last_scale || group == 0 {
                return None;
            }
            total = total.checked_add(group.checked_mul(scale)?)?;
            last_scale = scale;
            group = 0;
            pending = false;
            has_scale = true;
        } else {
            if pending {
                return None;
            }
            group = atom(token)?;
            pending = true;
        }
    }
    if !has_scale {
        return None;
    }
    total.checked_add(group)
}
pub fn format_digits(raw: &str, style: &DigitStyle) -> String {
    raw.chars()
        .map(|c| match style {
            DigitStyle::Bengali if c.is_ascii_digit() => {
                char::from_u32('০' as u32 + (c as u32 - '0' as u32)).unwrap_or(c)
            }
            DigitStyle::Latin if ('০'..='৯').contains(&c) => {
                char::from_u32('0' as u32 + (c as u32 - '০' as u32)).unwrap_or(c)
            }
            _ => c,
        })
        .collect()
}
fn grouped(n: u64) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    let split = s.len() - 3;
    let mut parts = Vec::new();
    let mut end = split;
    while end > 0 {
        let start = end.saturating_sub(2);
        parts.push(s[start..end].to_string());
        end = start;
    }
    parts.reverse();
    parts.push(s[split..].to_string());
    parts.join(",")
}
pub fn normalize(
    raw: &str,
    entries: &[Entry],
    style: &DigitStyle,
    commands: bool,
    snippets: &[Snippet],
) -> String {
    let raw = raw.trim();
    if commands {
        match raw.trim_end_matches(['।', '.']) {
            "নতুন লাইন" => return "\n".into(),
            "নতুন অনুচ্ছেদ" => return "\n\n".into(),
            _ => {}
        }
        if let Some(trigger) = raw.strip_prefix("স্নিপেট ") {
            if let Some(s) = snippets.iter().find(|s| s.trigger == trigger) {
                return s.text.clone();
            }
        }
    }
    let replaced = replace_aliases(raw, entries);
    // Explicit aliases are protected from number/digit rewriting.
    if entries
        .iter()
        .any(|e| e.enabled && !e.alias.is_empty() && raw == e.alias)
    {
        return replaced;
    }
    if matches!(style, DigitStyle::Preserve) {
        return replaced;
    }
    let (amount, suffix) = replaced
        .strip_suffix(" টাকা")
        .map(|a| (a, " টাকা"))
        .unwrap_or((&replaced, ""));
    if let Some(value) = integer_phrase(amount) {
        return format!("{}{suffix}", format_digits(&grouped(value), style));
    }
    // Preserve preferred terms by formatting only spans outside them.
    let mut out = String::new();
    let mut offset = 0;
    while offset < replaced.len() {
        let rest = &replaced[offset..];
        if let Some(e) = entries
            .iter()
            .filter(|e| e.enabled)
            .find(|e| rest.starts_with(&e.term))
        {
            out.push_str(&e.term);
            offset += e.term.len();
        } else {
            let c = rest.chars().next().expect("nonempty suffix");
            out.push_str(&format_digits(&c.to_string(), style));
            offset += c.len_utf8();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(alias: &str, term: &str) -> Entry {
        Entry {
            alias: alias.into(),
            term: term.into(),
            enabled: true,
        }
    }
    #[test]
    fn no_negation_or_repetition_rewrite() {
        assert_eq!(
            normalize("না না, সেটা না", &[], &DigitStyle::Preserve, false, &[]),
            "না না, সেটা না"
        );
    }
    #[test]
    fn alias_boundaries_and_no_chaining() {
        let rules = vec![e("সাবসড্রপ", "SubsDrop"), e("SubsDrop", "wrong")];
        assert_eq!(
            replace_aliases("সাবসড্রপ, সাবসড্রপের", &rules),
            "SubsDrop, সাবসড্রপের"
        );
    }
    #[test]
    fn longest_alias_wins() {
        assert_eq!(
            replace_aliases(
                "Pro Trainer IT",
                &[e("Pro", "P"), e("Pro Trainer IT", "PTIT")]
            ),
            "PTIT"
        );
    }
    #[test]
    fn supported_amounts() {
        for (input, expected) in [
            ("তিন লাখ বিশ হাজার টাকা", "৩,২০,০০০ টাকা"),
            ("এক লাখ পঞ্চাশ হাজার", "১,৫০,০০০"),
            ("চল্লিশ হাজার টাকা", "৪০,০০০ টাকা"),
        ] {
            assert_eq!(
                normalize(input, &[], &DigitStyle::Bengali, false, &[]),
                expected
            );
        }
    }
    #[test]
    fn unsafe_numbers_preserved() {
        for input in [
            "এক দুই শূন্য পাঁচ",
            "তিন থেকে পাঁচ হাজার",
            "পাঁচটা ফাইল",
            "আগামী শুক্রবার",
            "বিশ লাখ লাখ",
            "এক হাজার দুই লাখ",
        ] {
            assert_eq!(
                normalize(input, &[], &DigitStyle::Bengali, false, &[]),
                input
            );
        }
    }
    #[test]
    fn literal_command_unchanged() {
        assert_eq!(
            normalize("নতুন লাইন কথাটা লিখুন", &[], &DigitStyle::Preserve, true, &[]),
            "নতুন লাইন কথাটা লিখুন"
        );
        assert_eq!(
            normalize("নতুন লাইন", &[], &DigitStyle::Preserve, false, &[]),
            "নতুন লাইন"
        );
    }
    #[test]
    fn explicit_commands_and_snippets() {
        assert_eq!(
            normalize("নতুন লাইন", &[], &DigitStyle::Preserve, true, &[]),
            "\n"
        );
        let s = vec![Snippet {
            trigger: "ঠিকানা".into(),
            text: "Example address".into(),
        }];
        assert_eq!(
            normalize("স্নিপেট ঠিকানা", &[], &DigitStyle::Preserve, true, &s),
            "Example address"
        );
    }
    #[test]
    fn dictionary_conflicts_rejected() {
        assert!(validate_entries(&[e("x", "a"), e("x", "b")]).is_err());
    }
    #[test]
    fn code_switch_and_joiners_preserved() {
        let s = "client-এর সঙ্গে Next.js meeting, র‌্যাম";
        assert_eq!(normalize(s, &[], &DigitStyle::Preserve, false, &[]), s);
    }
    #[test]
    fn preferred_digits_protected() {
        assert_eq!(
            normalize(
                "GPT-5 123",
                &[e("", "GPT-5")],
                &DigitStyle::Bengali,
                false,
                &[]
            ),
            "GPT-5 ১২৩"
        );
    }
    #[test]
    fn editing_only_exact_bounded_commands() {
        assert_eq!(
            edit_owned("আজ ভালো দিন", "শেষ শব্দ মুছুন"),
            Some("আজ ভালো".into())
        );
        assert_eq!(edit_owned("one\ntwo", "শেষ লাইন মুছুন"), Some("one".into()));
        assert_eq!(edit_owned("abc", "শেষ শব্দ মুছুন কথাটা লিখুন"), None);
    }
}

/// A flag holds automatic insertion. Its absence is not an accuracy guarantee.
pub fn unexpected_script(text: &str, language: &str) -> bool {
    if language == "auto" {
        return false;
    }
    text.chars().filter(|c| c.is_alphanumeric()).any(|c| {
        let n = c as u32;
        let latin = c.is_ascii_alphanumeric()
            || (0x00c0..=0x024f).contains(&n)
            || (0x1e00..=0x1eff).contains(&n);
        let bengali = (0x0980..=0x09ff).contains(&n);
        !(latin || language == "bn" && bengali)
    })
}
#[cfg(test)]
mod script_tests {
    use super::*;
    #[test]
    fn mismatched_script_held_without_rewriting() {
        assert!(!unexpected_script("বাংলা client ১২৩ 🙂", "bn"));
        assert!(unexpected_script("తెలుగు", "bn"));
        assert!(unexpected_script("বাংলা", "en"));
        assert!(!unexpected_script("café 123", "en"));
        assert!(!unexpected_script("తెలుగు", "auto"));
    }
}
