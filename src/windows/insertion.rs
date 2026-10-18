//! Automatic insertion is restricted to identifiable native Edit/RichEdit controls.
//! Unknown web/rich controls hold the result for explicit Copy; no blind paste.
use super::wide;
use sha2::{Digest, Sha256};
use std::{mem::size_of, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::*,
    System::{DataExchange::*, Memory::*},
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
#[derive(Clone, Copy)]
pub struct Target {
    pub window: HWND,
    pub control: HWND,
    pub process: u32,
    pub thread: u32,
    pub caret: u32,
    pub digest: [u8; 32],
}
pub struct Undo {
    pub target: Target,
    pub before: String,
    pub after: String,
    pub start: u32,
    pub count: u32,
    pub text: String,
}
unsafe fn send(hwnd: HWND, message: u32, w: usize, l: isize) -> Result<usize, String> {
    let mut result = 0;
    let success = SendMessageTimeoutW(
        hwnd,
        message,
        w,
        l,
        SMTO_ABORTIFHUNG | SMTO_BLOCK,
        300,
        &mut result,
    );
    if success == 0 {
        Err("Target did not respond".into())
    } else {
        Ok(result)
    }
}
unsafe fn selection(hwnd: HWND) -> Result<(u32, u32), String> {
    let (mut a, mut b) = (0u32, 0u32);
    send(
        hwnd,
        0x00b0,
        (&mut a as *mut u32) as usize,
        (&mut b as *mut u32) as isize,
    )?;
    Ok((a, b))
}
unsafe fn field_text(hwnd: HWND) -> Result<String, String> {
    let n = send(hwnd, WM_GETTEXTLENGTH, 0, 0)?;
    if n > 32768 {
        return Err("Target is too large for verified insertion; use Copy".into());
    }
    let mut buf = vec![0u16; n + 1];
    let got = send(hwnd, WM_GETTEXT, buf.len(), buf.as_mut_ptr() as isize)?;
    if got > n {
        return Err("Target changed".into());
    }
    String::from_utf16(&buf[..got]).map_err(|_| "Target contains invalid Unicode".into())
}
pub unsafe fn snapshot() -> Result<Target, String> {
    let window = GetForegroundWindow();
    if window.is_null() {
        return Err("No target window".into());
    }
    let mut process = 0;
    let thread = GetWindowThreadProcessId(window, &mut process);
    let mut info: GUITHREADINFO = std::mem::zeroed();
    info.cbSize = size_of::<GUITHREADINFO>() as u32;
    if GetGUIThreadInfo(thread, &mut info) == 0 || info.hwndFocus.is_null() {
        return Err("Text field could not be identified; use Copy".into());
    }
    let control = info.hwndFocus;
    if IsWindowUnicode(control) == 0 {
        return Err("ANSI controls require Copy".into());
    }
    let mut buf = [0u16; 128];
    let n = GetClassNameW(control, buf.as_mut_ptr(), buf.len() as i32);
    let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]).to_lowercase();
    if class != "edit" && !class.starts_with("richedit") {
        return Err("This field requires Copy / manual paste".into());
    }
    let style = GetWindowLongW(control, GWL_STYLE) as u32;
    if style & 0x20 != 0 {
        return Err("Password fields are blocked".into());
    }
    if style & 0x800 != 0 || IsWindowEnabled(control) == 0 {
        return Err("Read-only field".into());
    }
    let (a, b) = selection(control)?;
    if a != b {
        return Err("Selected text will not be overwritten".into());
    }
    Ok(Target {
        window,
        control,
        process,
        thread,
        caret: a,
        digest: Sha256::digest(field_text(control)?.as_bytes()).into(),
    })
}
pub unsafe fn validate(t: Target) -> Result<(), String> {
    let current = snapshot()?;
    if !same_identity(current, t) {
        return Err("Focus changed; result held".into());
    }
    if current.caret != t.caret || current.digest != t.digest {
        return Err("Caret or field content changed; result held".into());
    }
    Ok(())
}
fn same_identity(current: Target, t: Target) -> bool {
    !(current.window != t.window
        || current.control != t.control
        || current.process != t.process
        || current.thread != t.thread)
}
pub unsafe fn type_unicode(t: Target, text: &str) -> Result<Undo, String> {
    validate(t)?;
    if [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
        .iter()
        .any(|key| GetAsyncKeyState(*key as i32) < 0)
    {
        return Err("Release shortcut modifiers before insertion".into());
    }
    let before = field_text(t.control)?;
    let (start, end) = selection(t.control)?;
    if start != end {
        return Err("Selection changed; result held".into());
    }
    let units: Vec<_> = text.encode_utf16().collect();
    if units.len() > 8000 {
        return Err("Long results require Copy".into());
    }
    let mut expected: Vec<_> = before.encode_utf16().collect();
    let at = start as usize;
    if text.contains('\0')
        || at > expected.len()
        || (at > 0
            && at < expected.len()
            && (0xd800..=0xdbff).contains(&expected[at - 1])
            && (0xdc00..=0xdfff).contains(&expected[at]))
    {
        return Err("Caret unavailable".into());
    }
    expected.splice(start as usize..start as usize, units.iter().copied());
    if text.contains(['\n', '\r']) && GetWindowLongW(t.control, GWL_STYLE) as u32 & 4 == 0 {
        return Err("Single-line fields require Copy for multiline results".into());
    }
    validate(t)?;
    let value = wide(text);
    send(t.control, 0x00c2, 1, value.as_ptr() as isize)?;
    // Verification is deferred by the caller until the target has processed input.
    Ok(Undo {
        target: t,
        before,
        after: String::from_utf16_lossy(&expected),
        start,
        count: units.len() as u32,
        text: text.to_string(),
    })
}
pub unsafe fn confirm(u: &mut Undo) -> bool {
    let Ok(target) = snapshot() else {
        return false;
    };
    let Ok(after) = field_text(u.target.control) else {
        return false;
    };
    let Some(count) = voicetotext::delivery::inserted_count(&u.before, &after, u.start, &u.text)
    else {
        return false;
    };
    if !same_identity(target, u.target) || target.caret != u.start + count {
        return false;
    }
    u.after = after;
    u.count = count;
    true
}
pub unsafe fn confirm_undo(u: &Undo) -> bool {
    snapshot().is_ok_and(|t| same_identity(t, u.target) && t.caret == u.start)
        && field_text(u.target.control).is_ok_and(|s| s == u.before)
}
pub unsafe fn undo(u: &Undo) -> Result<(), String> {
    if !snapshot().is_ok_and(|t| same_identity(t, u.target) && t.caret == u.start + u.count)
        || field_text(u.target.control)? != u.after
    {
        return Err("Target/text changed. Undo is unavailable; edit manually.".into());
    }
    send(
        u.target.control,
        0x00b1,
        u.start as usize,
        (u.start + u.count) as isize,
    )?;
    if GetForegroundWindow() != u.target.window
        || field_text(u.target.control)? != u.after
        || selection(u.target.control)? != (u.start, u.start + u.count)
    {
        return Err("Target changed before undo; no delete was sent".into());
    }
    let empty = wide("");
    send(u.target.control, 0x00c2, 1, empty.as_ptr() as isize)?;
    Ok(())
}
pub unsafe fn copy(owner: HWND, text: &str) -> Result<(), String> {
    let units = wide(text);
    let mem = GlobalAlloc(GMEM_MOVEABLE, units.len() * 2);
    if mem.is_null() {
        return Err("Clipboard memory unavailable".into());
    }
    let p = GlobalLock(mem) as *mut u16;
    if p.is_null() {
        GlobalFree(mem);
        return Err("Clipboard lock failed".into());
    }
    std::ptr::copy_nonoverlapping(units.as_ptr(), p, units.len());
    GlobalUnlock(mem);
    if OpenClipboard(owner) == 0 {
        GlobalFree(mem);
        return Err("Clipboard busy; retry Copy".into());
    }
    let success = EmptyClipboard() != 0 && !SetClipboardData(13, mem).is_null();
    CloseClipboard();
    if !success {
        GlobalFree(mem);
        return Err("Copy failed".into());
    }
    Ok(())
}
pub unsafe fn current_or(t: Option<Target>) -> Result<Target, String> {
    match t {
        Some(t) => {
            validate(t)?;
            Ok(t)
        }
        None => Err(
            "No verified recording target. Use Copy, or select a supported field through Insert."
                .into(),
        ),
    }
}
#[allow(dead_code)]
fn _null() -> HWND {
    null_mut()
}
