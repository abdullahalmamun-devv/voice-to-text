mod capture;
mod insertion;
use std::{
    collections::BTreeMap,
    mem::size_of,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use voicetotext::{
    audio,
    engine::{self, Transcript},
    model,
    session::{Phase, Session},
    storage::{self, Settings},
    text::{self, DigitStyle, Entry, Snippet},
    worker::{Request, Worker},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::*, Registry::*, RemoteDesktop::*, Threading::*},
    UI::{
        Controls::Dialogs::*, HiDpi::*, Input::KeyboardAndMouse::*, Shell::*,
        WindowsAndMessaging::*,
    },
};
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn error_box(s: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(s).as_ptr(),
            wide("VoiceToText").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
const TRAY_MSG: u32 = WM_APP + 1;
const HOTKEY: i32 = 1;
const CANCELKEY: i32 = 2;
thread_local! {static IN_CALLBACK:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
struct CallbackGuard;
impl CallbackGuard {
    fn enter() -> Option<Self> {
        IN_CALLBACK.with(|flag| if flag.replace(true) { None } else { Some(Self) })
    }
}
impl Drop for CallbackGuard {
    fn drop(&mut self) {
        IN_CALLBACK.with(|flag| flag.set(false));
    }
}
static DARK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static BACKGROUND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct Control {
    hwnd: HWND,
    page: i32,
    rect: (i32, i32, i32, i32),
}
struct App {
    window: HWND,
    overlay: HWND,
    font: HFONT,
    heading: HFONT,
    controls: BTreeMap<i32, Control>,
    page: i32,
    settings: Settings,
    dir: PathBuf,
    session: Session,
    capture: Option<capture::Capture>,
    hold_active: bool,
    worker: Worker,
    target: Option<insertion::Target>,
    undo: Option<insertion::Undo>,
    undo_pending: bool,
    confirm_at: Option<Instant>,
    transcript: Transcript,
    last_audio: Vec<f32>,
    last_wav: Option<String>,
    previous_result: String,
    started: Instant,
    last_preview: Instant,
    last_activity: Instant,
    pending: Option<Request>,
    armed: bool,
    armed_undo: bool,
    delivery: Option<bool>,
    tray: NOTIFYICONDATAW,
    devices: Vec<(u32, String)>,
    status: String,
}
unsafe fn window_text(h: HWND) -> String {
    let n = GetWindowTextLengthW(h).clamp(0, 100000);
    let mut s = vec![0u16; n as usize + 1];
    let got = GetWindowTextW(h, s.as_mut_ptr(), s.len() as i32);
    String::from_utf16_lossy(&s[..got.max(0) as usize])
}
unsafe fn file_dialog(owner: HWND, save: bool, filter: &str, extension: &str) -> Option<PathBuf> {
    let mut buf = [0u16; 32768];
    let filter = wide(filter);
    let ext = wide(extension);
    let mut o: OPENFILENAMEW = std::mem::zeroed();
    o.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    o.hwndOwner = owner;
    o.lpstrFile = buf.as_mut_ptr();
    o.nMaxFile = buf.len() as u32;
    o.lpstrFilter = filter.as_ptr();
    o.lpstrDefExt = ext.as_ptr();
    o.Flags = OFN_EXPLORER
        | OFN_NOCHANGEDIR
        | if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST
        };
    let ok = if save {
        GetSaveFileNameW(&mut o)
    } else {
        GetOpenFileNameW(&mut o)
    };
    if ok == 0 {
        None
    } else {
        let n = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        Some(PathBuf::from(String::from_utf16_lossy(&buf[..n])))
    }
}
impl App {
    unsafe fn control(
        &mut self,
        id: i32,
        page: i32,
        class: &str,
        label: &str,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) {
        let dpi = GetDpiForWindow(self.window).max(96) as i32;
        let scale = |n: i32| n * dpi / 96;
        let c = CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            scale(x),
            scale(y),
            scale(w),
            scale(h),
            self.window,
            id as usize as HMENU,
            GetModuleHandleW(null()),
            null(),
        );
        SendMessageW(c, WM_SETFONT, self.font as usize, 1);
        self.controls.insert(
            id,
            Control {
                hwnd: c,
                page,
                rect: (x, y, w, h),
            },
        );
    }
    unsafe fn label(&mut self, id: i32, page: i32, label: &str, x: i32, y: i32, w: i32) {
        self.control(id, page, "STATIC", label, 0, x, y, w, 30);
    }
    unsafe fn button(&mut self, id: i32, page: i32, label: &str, x: i32, y: i32, w: i32) {
        self.control(
            id,
            page,
            "BUTTON",
            label,
            WS_TABSTOP | BS_PUSHBUTTON as u32,
            x,
            y,
            w,
            36,
        );
    }
    unsafe fn edit(&mut self, id: i32, page: i32, x: i32, y: i32, w: i32, h: i32, multiline: bool) {
        self.control(
            id,
            page,
            "EDIT",
            "",
            WS_TABSTOP
                | WS_BORDER
                | if multiline {
                    ES_MULTILINE as u32 | ES_AUTOVSCROLL as u32 | WS_VSCROLL | ES_WANTRETURN as u32
                } else {
                    ES_AUTOHSCROLL as u32
                },
            x,
            y,
            w,
            h,
        );
        SendMessageW(self.h(id), 0x00c5, 80000, 0);
    }
    unsafe fn check(&mut self, id: i32, page: i32, label: &str, x: i32, y: i32, w: i32, on: bool) {
        self.control(
            id,
            page,
            "BUTTON",
            label,
            WS_TABSTOP | BS_AUTOCHECKBOX as u32,
            x,
            y,
            w,
            30,
        );
        SendMessageW(self.h(id), BM_SETCHECK, usize::from(on), 0);
    }
    unsafe fn combo(
        &mut self,
        id: i32,
        page: i32,
        items: &[&str],
        x: i32,
        y: i32,
        w: i32,
        selected: usize,
    ) {
        self.control(
            id,
            page,
            "COMBOBOX",
            "",
            WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            x,
            y,
            w,
            250,
        );
        for item in items {
            SendMessageW(self.h(id), CB_ADDSTRING, 0, wide(item).as_ptr() as isize);
        }
        SendMessageW(self.h(id), CB_SETCURSEL, selected, 0);
    }
    fn h(&self, id: i32) -> HWND {
        self.controls.get(&id).map(|c| c.hwnd).unwrap_or(null_mut())
    }
    unsafe fn set(&self, id: i32, s: &str) {
        SetWindowTextW(self.h(id), wide(s).as_ptr());
    }
    unsafe fn checked(&self, id: i32) -> bool {
        SendMessageW(self.h(id), BM_GETCHECK, 0, 0) == 1
    }
    unsafe fn index(&self, id: i32) -> usize {
        SendMessageW(self.h(id), CB_GETCURSEL, 0, 0).max(0) as usize
    }
    unsafe fn text(&self, id: i32) -> String {
        window_text(self.h(id))
    }
    unsafe fn status(&mut self, text: &str) {
        if self.status != text {
            self.status = text.into();
            self.set(50, text);
            SetWindowTextW(self.overlay, wide(text).as_ptr());
        }
    }
    unsafe fn show_page(&mut self, page: i32) {
        self.page = page;
        for c in self.controls.values() {
            ShowWindow(
                c.hwnd,
                if c.page < 0 || c.page == page {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
        }
        if page == 3 {
            self.refresh_history();
        }
    }
    unsafe fn layout(&mut self) {
        self.label(1, -1, "VoiceToText", 28, 22, 730);
        SendMessageW(self.h(1), WM_SETFONT, self.heading as usize, 1);
        self.label(
            2,
            -1,
            "Offline Bengali dictation · V1 + V2 alpha",
            28,
            53,
            730,
        );
        for (i, label) in [
            "কথা থেকে লেখা",
            "Preferences",
            "Dictionary & snippets",
            "Local history",
        ]
        .iter()
        .enumerate()
        {
            self.button(900 + i as i32, -1, label, 28 + i as i32 * 187, 92, 180);
        }
        self.label(50, -1, "কথা বলার জন্য প্রস্তুত", 28, 145, 750);
        self.label(
            3,
            0,
            "প্রথমবার: microphone নির্বাচন করুন, shortcut দিয়ে কথা বলুন, তারপর লেখা যাচাই করুন।",
            28,
            191,
            750,
        );
        let names: Vec<String> = self.devices.iter().map(|x| x.1.clone()).collect();
        let device_names: Vec<_> = names.iter().map(String::as_str).collect();
        let idx = self
            .devices
            .iter()
            .position(|x| x.0 == self.settings.microphone)
            .unwrap_or(0);
        self.combo(10, 0, &device_names, 28, 225, 445, idx);
        self.button(100, 0, "শুরু / বন্ধ", 492, 222, 130);
        self.button(101, 0, "Cancel", 636, 222, 132);
        self.label(4, 0, "Result — প্রয়োজন হলে সম্পাদনা করুন", 28, 276, 740);
        self.edit(12, 0, 28, 311, 740, 252, true);
        self.button(102, 0, "Copy", 28, 580, 110);
        self.button(103, 0, "Insert…", 150, 580, 110);
        self.button(104, 0, "Retry", 272, 580, 110);
        self.button(108, 0, "Discard", 394, 580, 110);
        self.button(110, 0, "Undo…", 516, 580, 110);
        self.button(109, 0, "Open WAV", 28, 631, 130);
        self.button(105, 0, "Export TXT", 170, 631, 130);
        self.button(106, 0, "Markdown", 312, 631, 130);
        self.button(107, 0, "SRT (raw)", 454, 631, 130);
        self.label(
            5,
            0,
            "Insert / Undo: ক্লিক করুন, target field নির্বাচন করুন, তারপর shortcut চাপুন।",
            28,
            691,
            745,
        );

        self.label(211, 1, "Language", 28, 195, 180);
        self.combo(
            20,
            1,
            &["বাংলা", "English", "বাংলা + English"],
            235,
            190,
            530,
            match self.settings.language.as_str() {
                "en" => 1,
                "auto" => 2,
                _ => 0,
            },
        );
        self.label(212, 1, "Number style", 28, 238, 180);
        self.combo(
            21,
            1,
            &["Keep recognized spelling", "বাংলা সংখ্যা", "Latin digits"],
            235,
            235,
            530,
            match self.settings.digits {
                DigitStyle::Preserve => 0,
                DigitStyle::Bengali => 1,
                DigitStyle::Latin => 2,
            },
        );
        self.check(
            22,
            1,
            "Hold shortcut to talk",
            28,
            282,
            330,
            self.settings.hold_to_talk,
        );
        self.check(
            23,
            1,
            "Review before insertion",
            408,
            282,
            350,
            self.settings.review,
        );
        self.check(
            24,
            1,
            "Explicit formatting / snippet commands",
            28,
            324,
            360,
            self.settings.commands,
        );
        self.check(
            25,
            1,
            "Live draft preview (experimental)",
            408,
            324,
            360,
            self.settings.live_preview,
        );
        self.check(
            26,
            1,
            "Save local transcript history",
            28,
            366,
            330,
            self.settings.history_enabled,
        );
        self.combo(
            27,
            1,
            &["7 days", "30 days", "Manual retention"],
            408,
            365,
            355,
            match self.settings.retention_days {
                30 => 1,
                0 => 2,
                _ => 0,
            },
        );
        self.label(213, 1, "Unload model after idle", 28, 414, 200);
        self.combo(
            28,
            1,
            &["30 seconds", "5 minutes", "15 minutes", "Keep model ready"],
            235,
            411,
            530,
            match self.settings.saver_seconds {
                30 => 0,
                900 => 2,
                0 => 3,
                _ => 1,
            },
        );
        self.label(214, 1, "Shortcut key", 28, 459, 180);
        self.combo(
            29,
            1,
            &["Space", "F8", "F9", "F10", "D"],
            235,
            456,
            150,
            match self.settings.hotkey {
                0x77 => 1,
                0x78 => 2,
                0x79 => 3,
                0x44 => 4,
                _ => 0,
            },
        );
        for (id, label, bit, x) in [
            (30, "Ctrl", 2, 405),
            (31, "Alt", 1, 495),
            (32, "Shift", 4, 585),
            (33, "Win", 8, 675),
        ] {
            self.check(id, 1, label, x, 453, 90, self.settings.modifiers & bit != 0);
        }
        self.check(
            34,
            1,
            "Start at Windows sign-in (opt-in)",
            28,
            506,
            370,
            self.settings.startup,
        );
        self.combo(
            35,
            1,
            &["System theme", "Light", "Dark"],
            408,
            505,
            355,
            match self.settings.theme.as_str() {
                "light" => 1,
                "dark" => 2,
                _ => 0,
            },
        );
        self.button(200, 1, "Save preferences", 28, 560, 215);
        self.button(201, 1, "Import settings", 255, 560, 170);
        self.button(202, 1, "Export settings", 437, 560, 170);
        self.button(204, 1, "Import local model", 28, 614, 215);
        self.button(203, 1, "Export diagnostics", 255, 614, 215);
        self.label(
            215,
            1,
            "No account · No network client · No automatic audio files or updates",
            28,
            676,
            735,
        );

        self.label(
            320,
            2,
            "Preferred terms / exact aliases (maximum 500)",
            28,
            195,
            720,
        );
        self.control(
            40,
            2,
            "LISTBOX",
            "",
            WS_TABSTOP | WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
            28,
            235,
            355,
            198,
        );
        self.label(321, 2, "Spoken alias (optional)", 408, 228, 350);
        self.edit(41, 2, 408, 265, 355, 35, false);
        self.label(322, 2, "Preferred spelling", 408, 313, 350);
        self.edit(42, 2, 408, 350, 355, 35, false);
        self.button(303, 2, "Add / update", 408, 400, 165);
        self.button(300, 2, "Remove selected", 588, 400, 175);
        self.button(301, 2, "Import JSON", 28, 445, 165);
        self.button(302, 2, "Export JSON", 207, 445, 175);
        self.button(304, 2, "Save current correction", 408, 445, 355);
        self.label(
            323,
            2,
            "Snippets — trigger is used only with an enabled explicit command",
            28,
            500,
            730,
        );
        self.control(
            43,
            2,
            "LISTBOX",
            "",
            WS_TABSTOP | WS_BORDER | WS_VSCROLL,
            28,
            540,
            245,
            130,
        );
        self.edit(44, 2, 292, 540, 185, 35, false);
        self.edit(45, 2, 492, 540, 270, 75, true);
        self.button(310, 2, "Save snippet", 292, 634, 185);
        self.button(311, 2, "Remove snippet", 492, 634, 270);
        self.refresh_dictionary();

        self.label(420,3,"History is opt-in. Audio is never included. Turning history off preserves existing entries.",28,195,740);
        self.control(
            60,
            3,
            "LISTBOX",
            "",
            WS_TABSTOP | WS_BORDER | WS_VSCROLL,
            28,
            235,
            740,
            160,
        );
        self.edit(61, 3, 28, 415, 740, 200, true);
        SendMessageW(self.h(61), 0x00cf, 1, 0);
        self.button(400, 3, "Open selected", 28, 637, 170);
        self.button(401, 3, "Copy", 210, 637, 100);
        self.button(402, 3, "Export TXT", 322, 637, 130);
        self.button(404, 3, "Delete all", 600, 637, 168);
        self.show_page(0);
    }
    unsafe fn theme(&self) {
        let dark = match self.settings.theme.as_str() {
            "dark" => true,
            "light" => false,
            _ => {
                let (mut value, mut length) = (1u32, 4u32);
                let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
                let key = wide("AppsUseLightTheme");
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    sub.as_ptr(),
                    key.as_ptr(),
                    RRF_RT_REG_DWORD,
                    null_mut(),
                    (&mut value as *mut u32) as *mut _,
                    &mut length,
                );
                value == 0
            }
        };
        DARK.store(dark, Ordering::Relaxed);
        let brush = CreateSolidBrush(if dark { 0x00262120 } else { 0x00ffffff });
        let old = BACKGROUND.swap(brush as usize, Ordering::Relaxed);
        if old != 0 {
            DeleteObject(old as HBRUSH);
        }
        for c in self.controls.values() {
            windows_sys::Win32::UI::Controls::SetWindowTheme(
                c.hwnd,
                wide(if dark {
                    "DarkMode_Explorer"
                } else {
                    "Explorer"
                })
                .as_ptr(),
                null(),
            );
            InvalidateRect(c.hwnd, null(), 1);
        }
        InvalidateRect(self.window, null(), 1);
        InvalidateRect(self.overlay, null(), 1);
    }
    unsafe fn dpi(&mut self, dpi: u32, rect: *const RECT) {
        if !rect.is_null() {
            let r = *rect;
            SetWindowPos(
                self.window,
                null_mut(),
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        let font = CreateFontW(
            -17 * dpi as i32 / 96,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide("Nirmala UI").as_ptr(),
        );
        let old = self.font;
        self.font = font;
        let old_heading = self.heading;
        self.heading = CreateFontW(
            -27 * dpi as i32 / 96,
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide("Segoe UI").as_ptr(),
        );
        for (id, c) in &self.controls {
            let (x, y, w, h) = c.rect;
            let scale = |n: i32| n * dpi as i32 / 96;
            MoveWindow(c.hwnd, scale(x), scale(y), scale(w), scale(h), 1);
            SendMessageW(
                c.hwnd,
                WM_SETFONT,
                if *id == 1 { self.heading } else { font } as usize,
                1,
            );
        }
        DeleteObject(old);
        DeleteObject(old_heading);
    }
    unsafe fn refresh_dictionary(&self) {
        SendMessageW(self.h(40), LB_RESETCONTENT, 0, 0);
        for e in &self.settings.entries {
            let label = if e.alias.is_empty() {
                e.term.clone()
            } else {
                format!("{} → {}", e.alias, e.term)
            };
            SendMessageW(self.h(40), LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        }
        SendMessageW(self.h(43), LB_RESETCONTENT, 0, 0);
        for s in &self.settings.snippets {
            SendMessageW(
                self.h(43),
                LB_ADDSTRING,
                0,
                wide(&s.trigger).as_ptr() as isize,
            );
        }
    }
    unsafe fn refresh_history(&mut self) {
        match storage::history(&self.dir, self.settings.retention_days) {
            Ok(items) => {
                SendMessageW(self.h(60), LB_RESETCONTENT, 0, 0);
                for item in items {
                    let preview: String = item.text.chars().take(65).collect();
                    let row = format!("{} · {}", item.timestamp, preview);
                    SendMessageW(self.h(60), LB_ADDSTRING, 0, wide(&row).as_ptr() as isize);
                }
            }
            Err(e) => self.status(&e),
        }
    }
    unsafe fn persist(&mut self) -> bool {
        if let Err(e) = storage::save_settings(&self.dir, &self.settings) {
            self.status(&format!("Save failed: {e}. Changes apply in memory only; retry or restart to recover disk settings."));
            return false;
        }
        true
    }
    unsafe fn hotkey(&mut self) {
        if self.armed {
            self.armed = false;
            if self.armed_undo {
                self.delivery = Some(true);
                self.armed_undo = false;
                return;
            }
            self.target = insertion::snapshot().ok();
            self.delivery = Some(false);
            return;
        }
        match self.session.phase {
            Phase::Listening => {
                if !self.settings.hold_to_talk {
                    self.stop();
                }
            }
            Phase::Processing => self.status("লেখা তৈরি হচ্ছে… Cancel দিয়ে বাতিল করতে পারেন।"),
            _ => {
                self.start();
                self.hold_active = self.settings.hold_to_talk;
            }
        }
    }
    unsafe fn start(&mut self) {
        if self.settings.model_path.is_empty() {
            self.status("Local model প্রয়োজন। Preferences → Import local model।");
            return;
        }
        self.hold_active = false;
        self.previous_result = self.text(12);
        match insertion::snapshot() {
            Ok(t) => self.target = Some(t),
            Err(e) => {
                if e == "Password fields are blocked" {
                    self.status(&e);
                    return;
                }
                self.target = None;
            }
        }
        self.undo = None;
        self.undo_pending = false;
        self.transcript = Transcript::default();
        self.last_audio.clear();
        self.last_wav = None;
        self.pending = None;
        let index = self.index(10);
        let device = self.devices.get(index).map(|x| x.0).unwrap_or(u32::MAX);
        match capture::Capture::start(device) {
            Ok(c) => {
                if self.session.start().is_err() {
                    drop(c);
                    return;
                }
                self.capture = Some(c);
                self.started = Instant::now();
                self.last_preview = self.started;
                self.last_activity = self.started;
                self.set(12, "");
                self.status("শুনছি…");
                ShowWindow(self.overlay, SW_SHOWNOACTIVATE);
                RegisterHotKey(self.window, CANCELKEY, MOD_NOREPEAT, VK_ESCAPE as u32);
            }
            Err(e) => self.status(&e),
        }
    }
    unsafe fn stop(&mut self) {
        let Some(c) = self.capture.take() else {
            return;
        };
        self.last_audio = c.stop();
        if self.session.stop().is_err() {
            return;
        }
        self.status("লেখা তৈরি হচ্ছে…");
        SetWindowTextW(self.overlay, wide("লেখা তৈরি হচ্ছে…").as_ptr());
        self.pending = Some(self.request(self.last_audio.clone(), false));
        self.submit_pending();
    }
    fn request(&self, samples: Vec<f32>, preview: bool) -> Request {
        Request {
            session_id: self.session.id,
            model: self.settings.model_path.clone(),
            samples,
            wav_path: if preview { None } else { self.last_wav.clone() },
            no_timestamps: false,
            language: self.settings.language.clone(),
            prompt: text::prompt(&self.settings.entries),
            preview,
        }
    }
    unsafe fn submit_pending(&mut self) {
        if !self.worker.busy.load(Ordering::SeqCst) {
            if let Some(r) = self.pending.take() {
                match self.worker.tx.try_send(r) {
                    Ok(()) => {}
                    Err(std::sync::mpsc::TrySendError::Full(r)) => self.pending = Some(r),
                    Err(_) => {
                        self.session.fail(self.session.id);
                        self.status("Worker unavailable. Restart the application.");
                    }
                }
            }
        }
    }
    unsafe fn cancel(&mut self) {
        self.capture.take();
        self.session.cancel();
        self.worker.cancel();
        self.pending = None;
        self.last_audio.clear();
        self.last_wav = None;
        self.previous_result.clear();
        self.target = None;
        self.undo = None;
        self.armed = false;
        self.armed_undo = false;
        self.delivery = None;
        self.confirm_at = None;
        self.transcript = Transcript::default();
        self.set(12, "");
        ShowWindow(self.overlay, SW_HIDE);
        UnregisterHotKey(self.window, CANCELKEY);
        self.status("বাতিল করা হয়েছে। Microphone বন্ধ।");
    }
    unsafe fn try_insert(&mut self) {
        let text = self.text(12);
        if text.is_empty() {
            self.status("No result to insert");
            return;
        }
        match insertion::current_or(self.target).and_then(|t| insertion::type_unicode(t, &text)) {
            Ok(undo) => {
                self.undo = Some(undo);
                self.confirm_at = Some(Instant::now() + Duration::from_millis(350));
                self.status("Delivery attempted; checking target…");
            }
            Err(e) => {
                self.session.phase = Phase::Held;
                self.status(&e);
                ShowWindow(self.window, SW_SHOW);
                SetForegroundWindow(self.window);
            }
        }
    }
    unsafe fn tick(&mut self) {
        if self.delivery.is_some()
            && [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
                .iter()
                .all(|key| GetAsyncKeyState(*key as i32) >= 0)
        {
            let undo = self.delivery.take().unwrap_or(false);
            if undo {
                let result = self
                    .undo
                    .as_ref()
                    .ok_or_else(|| "No verified insertion".to_string())
                    .and_then(|u| insertion::undo(u));
                match result {
                    Ok(()) => {
                        self.undo_pending = true;
                        self.confirm_at = Some(Instant::now() + Duration::from_millis(350));
                        self.status("Undo attempted; checking target…");
                    }
                    Err(e) => self.status(&e),
                }
            } else {
                self.try_insert();
            }
        }
        if self.session.phase == Phase::Listening {
            let elapsed = self.started.elapsed().as_secs();
            let level = self.capture.as_ref().map(|c| c.level()).unwrap_or(0.);
            let limit = self
                .capture
                .as_ref()
                .is_some_and(|c| c.len() >= audio::MAX_TURN);
            let status = format!(
                "শুনছি… {elapsed}s · level {:.0}%{}",
                (level * 300.).min(100.),
                if elapsed >= 90 {
                    " · 120s limit approaching"
                } else {
                    ""
                }
            );
            self.status(&status);
            if limit || elapsed >= 120 {
                self.stop();
            } else if self.hold_active && GetAsyncKeyState(self.settings.hotkey as i32) >= 0 {
                self.stop();
            } else if self.settings.live_preview
                && self.last_preview.elapsed() >= Duration::from_secs(5)
                && !self.worker.busy.load(Ordering::SeqCst)
            {
                if let Some(c) = &self.capture {
                    let r = self.request(c.samples(), true);
                    if self.worker.tx.try_send(r).is_ok() {
                        self.last_preview = Instant::now();
                    }
                }
            }
        }
        self.submit_pending();
        while let Ok(response) = self.worker.rx.try_recv() {
            if response.session_id != self.session.id {
                continue;
            }
            if response.preview {
                if self.session.phase == Phase::Listening {
                    if let Some(t) = response.transcript {
                        self.set(12, &format!("Draft / খসড়া: {}", t.text));
                    }
                }
                continue;
            }
            ShowWindow(self.overlay, SW_HIDE);
            UnregisterHotKey(self.window, CANCELKEY);
            self.last_activity = Instant::now();
            if let Some(e) = response.error {
                self.session.fail(response.session_id);
                self.status(&e);
                continue;
            }
            if let Some(t) = response.transcript {
                if !self.session.finish(response.session_id, !t.text.is_empty()) {
                    continue;
                }
                let edit = if self.settings.commands {
                    text::edit_owned(&self.previous_result, &t.text)
                } else {
                    None
                };
                let edited = edit.is_some();
                let output = edit.unwrap_or_else(|| {
                    text::normalize(
                        &t.text,
                        &self.settings.entries,
                        &self.settings.digits,
                        self.settings.commands,
                        &self.settings.snippets,
                    )
                });
                self.transcript = t;
                self.set(12, &output);
                if output.is_empty() {
                    self.status("কথা পাওয়া যায়নি। আবার চেষ্টা করুন।");
                } else {
                    self.status(&format!(
                        "লেখা প্রস্তুত · {:.1}s processing · যাচাই করুন",
                        self.transcript.decode_ms as f64 / 1000.
                    ));
                    if let Err(e) = storage::append_history(&self.dir, &self.settings, &output) {
                        self.status(&format!("Result ready; history could not save: {e}"));
                    }
                    if !self.settings.review
                        && !edited
                        && !self.transcript.review_required
                        && !self.transcript.experimental_model
                    {
                        self.try_insert();
                    } else {
                        ShowWindow(self.window, SW_SHOW);
                        SetForegroundWindow(self.window);
                    }
                }
            }
        }
        if self.confirm_at.is_some_and(|t| Instant::now() >= t) {
            self.confirm_at = None;
            if self.undo_pending {
                self.undo_pending = false;
                let success = self
                    .undo
                    .as_ref()
                    .is_some_and(|u| insertion::confirm_undo(u));
                self.undo = None;
                self.status(if success {
                    "Undo verified."
                } else {
                    "Undo unconfirmed. Check the field; no repeated keystrokes will be sent."
                });
            } else if self.undo.as_mut().is_some_and(|u| insertion::confirm(u)) {
                self.session.phase = Phase::Ready;
                self.status("লেখা বসানো হয়েছে।");
            } else {
                self.undo = None;
                self.session.phase = Phase::Held;
                self.status("Delivery unconfirmed. Check target before any manual paste; no automatic retry.");
            }
        }
        if self.settings.saver_seconds != 0
            && !matches!(self.session.phase, Phase::Listening | Phase::Processing)
            && self.last_activity.elapsed().as_secs() >= self.settings.saver_seconds
        {
            self.worker.cancel();
            self.last_activity = Instant::now();
        }
    }
    unsafe fn save_preferences(&mut self) {
        if matches!(self.session.phase, Phase::Listening | Phase::Processing) {
            self.status("Finish or cancel the current session before changing preferences.");
            return;
        }
        let mut s = self.settings.clone();
        s.language = ["bn", "en", "auto"][self.index(20).min(2)].into();
        s.digits = [DigitStyle::Preserve, DigitStyle::Bengali, DigitStyle::Latin]
            [self.index(21).min(2)]
        .clone();
        s.hold_to_talk = self.checked(22);
        s.review = self.checked(23);
        s.commands = self.checked(24);
        s.live_preview = self.checked(25);
        s.history_enabled = self.checked(26);
        s.retention_days = [7, 30, 0][self.index(27).min(2)];
        s.saver_seconds = [30, 300, 900, 0][self.index(28).min(3)];
        s.hotkey = [0x20, 0x77, 0x78, 0x79, 0x44][self.index(29).min(4)];
        s.modifiers = 0;
        for (id, bit) in [(30, 2), (31, 1), (32, 4), (33, 8)] {
            if self.checked(id) {
                s.modifiers |= bit;
            }
        }
        s.startup = self.checked(34);
        s.microphone = self
            .devices
            .get(self.index(10))
            .map(|x| x.0)
            .unwrap_or(u32::MAX);
        s.theme = ["system", "light", "dark"][self.index(35).min(2)].into();
        if let Err(e) = s.validate() {
            self.status(&e);
            return;
        }
        UnregisterHotKey(self.window, HOTKEY);
        if RegisterHotKey(self.window, HOTKEY, s.modifiers | MOD_NOREPEAT, s.hotkey) == 0 {
            RegisterHotKey(
                self.window,
                HOTKEY,
                self.settings.modifiers | MOD_NOREPEAT,
                self.settings.hotkey,
            );
            self.status("Shortcut conflict. Choose another combination.");
            return;
        }
        self.settings = s;
        if !self.persist() {
            return;
        }
        self.theme();
        if let Err(e) = self.startup() {
            self.status(&e);
            return;
        }
        self.status("Preferences saved locally. Shortcut is ready.");
    }
    unsafe fn startup(&self) -> Result<(), String> {
        // Per-user Startup shortcut is a small local .cmd. No administrator or network access.
        let path = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or("Windows profile directory unavailable")?
            .join("Microsoft/Windows/Start Menu/Programs/Startup/VoiceToText.cmd");
        if self.settings.startup {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let exe = exe.to_str().ok_or("Unsupported executable path")?;
            if exe.contains(['%', '\n', '\r', '"']) {
                return Err("Startup requires a path without shell-special characters".into());
            }
            storage::atomic_write(
                &path,
                format!("@echo off\r\nchcp 65001 >nul\r\nstart \"\" \"{exe}\" --hidden\r\n")
                    .as_bytes(),
            )
        } else {
            if path.exists() {
                std::fs::remove_file(path).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
    }
    unsafe fn action(&mut self, id: i32) {
        if matches!(self.session.phase, Phase::Listening | Phase::Processing)
            && matches!(id, 103 | 104 | 110)
        {
            self.status("Finish or cancel the current recording before Insert, Retry or Undo.");
            return;
        }
        if matches!(self.session.phase, Phase::Listening | Phase::Processing)
            && matches!(id,105..=107|201..=204|301..=302|402|404)
        {
            self.status("Finish or cancel the current session before opening a file dialog.");
            return;
        }
        match id {
            900..=903 => self.show_page(id - 900),
            100 => match self.session.phase {
                Phase::Listening => self.stop(),
                Phase::Processing => self.status("Processing…"),
                _ => self.start(),
            },
            101 | 108 => self.cancel(),
            102 => {
                let text = self.text(12);
                match insertion::copy(self.window, &text) {
                    Ok(()) => self.status("Copied. Paste manually into the intended field."),
                    Err(e) => self.status(&e),
                }
            }
            103 => {
                if !self.text(12).is_empty() {
                    self.armed = true;
                    self.armed_undo = false;
                    self.undo = None;
                    self.status("Insert armed. Select target and press the shortcut.");
                    ShowWindow(self.window, SW_HIDE);
                }
            }
            110 => {
                if self.undo.is_some() {
                    self.armed = true;
                    self.armed_undo = true;
                    self.status("Undo armed. Select original target and press the shortcut.");
                    ShowWindow(self.window, SW_HIDE);
                } else {
                    self.status("No verified insertion to undo");
                }
            }
            104 => {
                if self.last_audio.is_empty() && self.last_wav.is_none() {
                    self.status("No audio retained for Retry");
                    return;
                }
                if self.session.start().is_ok() {
                    self.session.stop().ok();
                    self.pending = Some(self.request(self.last_audio.clone(), false));
                    RegisterHotKey(self.window, CANCELKEY, MOD_NOREPEAT, VK_ESCAPE as u32);
                    self.status("Retrying local transcription…");
                    self.submit_pending();
                }
            }
            105 | 106 | 107 => self.export(id),
            109 => {
                if matches!(self.session.phase, Phase::Listening | Phase::Processing) {
                    self.status("Finish current session first");
                    return;
                }
                if let Some(p) = file_dialog(self.window, false, "PCM WAV\0*.wav\0\0", "wav") {
                    self.last_wav = Some(p.to_string_lossy().into_owned());
                    self.last_audio.clear();
                    self.session.start().ok();
                    self.session.stop().ok();
                    self.target = None;
                    self.pending = Some(self.request(Vec::new(), false));
                    RegisterHotKey(self.window, CANCELKEY, MOD_NOREPEAT, VK_ESCAPE as u32);
                    self.status("Local WAV transcription… Cancel remains available.");
                    self.submit_pending();
                }
            }
            200 => self.save_preferences(),
            201 => {
                if let Some(p) =
                    file_dialog(self.window, false, "Settings JSON\0*.json\0\0", "json")
                {
                    let r = std::fs::read(&p)
                        .map_err(|e| e.to_string())
                        .and_then(|b| {
                            if b.len() > 1_000_000 {
                                return Err("Settings file too large".into());
                            }
                            serde_json::from_slice::<Settings>(&b).map_err(|e| e.to_string())
                        })
                        .and_then(|s| {
                            s.validate()?;
                            storage::save_settings(&self.dir, &s)
                        });
                    match r {
                        Ok(()) => {
                            self.status("Settings imported. Restart the application to apply.")
                        }
                        Err(e) => self.status(&e),
                    }
                }
            }
            202 => {
                if let Some(p) = file_dialog(self.window, true, "JSON\0*.json\0\0", "json") {
                    match serde_json::to_vec_pretty(&self.settings)
                        .map_err(|e| e.to_string())
                        .and_then(|b| storage::atomic_write(&p, &b))
                    {
                        Ok(()) => self.status("Settings and dictionary exported by your request."),
                        Err(e) => self.status(&e),
                    }
                }
            }
            203 => {
                if let Some(p) = file_dialog(self.window, true, "JSON\0*.json\0\0", "json") {
                    let info = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"platform":"Windows x64","worker_busy":self.worker.busy.load(Ordering::SeqCst),"last_decode_ms":self.transcript.decode_ms,"network_clients":"none in product source","source_runtime":"whisper.cpp via whisper-rs 0.15.1","contains_audio":false,"contains_transcript":false,"real_windows_qa":"pending"});
                    match storage::atomic_write(&p,info.to_string().as_bytes()){Ok(())=>self.status("Diagnostics exported; no audio, text, dictionary or user paths included."),Err(e)=>self.status(&e)}
                }
            }
            204 => {
                if matches!(self.session.phase, Phase::Listening | Phase::Processing) {
                    self.status("Finish current session first");
                    return;
                }
                if let Some(p) = file_dialog(
                    self.window,
                    false,
                    "Local speech model\0*.bin;*.vtt-model.json\0\0",
                    "bin",
                ) {
                    match model::inspect(&p) {
                        Ok(_) => {
                            self.worker.cancel();
                            self.settings.model_path = p.to_string_lossy().into_owned();
                            if !self.persist() {
                                return;
                            }
                            self.status("Local model selected. Imported packs are not evaluated; test a short sample.");
                        }
                        Err(e) => self.status(&e),
                    }
                }
            }
            303 => {
                let term = self.text(42).trim().to_string();
                let alias = self.text(41).trim().to_string();
                let mut entries = self.settings.entries.clone();
                let entry = Entry {
                    term,
                    alias: alias.clone(),
                    enabled: true,
                };
                if let Some(old) = entries.iter_mut().find(|e| {
                    if alias.is_empty() {
                        e.alias.is_empty() && e.term == entry.term
                    } else {
                        e.alias == alias
                    }
                }) {
                    *old = entry;
                } else {
                    entries.push(entry);
                }
                match text::validate_entries(&entries) {
                    Ok(()) => {
                        self.settings.entries = entries;
                        if !self.persist() {
                            return;
                        }
                        self.refresh_dictionary();
                        self.set(41, "");
                        self.set(42, "");
                        self.status("Dictionary saved");
                    }
                    Err(e) => self.status(&e),
                }
            }
            300 => {
                let i = SendMessageW(self.h(40), LB_GETCURSEL, 0, 0);
                if i >= 0 && (i as usize) < self.settings.entries.len() {
                    self.settings.entries.remove(i as usize);
                    if !self.persist() {
                        return;
                    }
                    self.refresh_dictionary();
                }
            }
            301 => {
                if let Some(p) =
                    file_dialog(self.window, false, "Dictionary JSON\0*.json\0\0", "json")
                {
                    let r = std::fs::read(&p)
                        .map_err(|e| e.to_string())
                        .and_then(|b| {
                            if b.len() > 500000 {
                                return Err("Dictionary file too large".into());
                            }
                            serde_json::from_slice::<Vec<Entry>>(&b).map_err(|e| e.to_string())
                        })
                        .and_then(|v| {
                            text::validate_entries(&v)?;
                            Ok(v)
                        });
                    match r {
                        Ok(v) => {
                            self.settings.entries = v;
                            if !self.persist() {
                                return;
                            }
                            self.refresh_dictionary();
                            self.status("Dictionary imported. Existing entries replaced.");
                        }
                        Err(e) => self.status(&e),
                    }
                }
            }
            302 => {
                if let Some(p) =
                    file_dialog(self.window, true, "Dictionary JSON\0*.json\0\0", "json")
                {
                    match serde_json::to_vec_pretty(&self.settings.entries)
                        .map_err(|e| e.to_string())
                        .and_then(|b| storage::atomic_write(&p, &b))
                    {
                        Ok(()) => self.status("Dictionary exported"),
                        Err(e) => self.status(&e),
                    }
                }
            }
            304 => {
                let alias = self.transcript.text.clone();
                let term = self.text(12);
                if alias.is_empty()
                    || alias == term
                    || alias.chars().count() > 160
                    || term.chars().count() > 160
                {
                    self.status("Edit a short result first (maximum 160 characters). A visible exact phrase rule will be proposed.");
                } else {
                    self.set(41, &alias);
                    self.set(42, &term);
                    self.status(
                        "Correction proposed. Click Add / update to confirm this exact alias.",
                    );
                }
            }
            310 => {
                let mut proposed = self.settings.clone();
                let s = Snippet {
                    trigger: self.text(44).trim().into(),
                    text: self.text(45),
                };
                if let Some(old) = proposed
                    .snippets
                    .iter_mut()
                    .find(|old| old.trigger == s.trigger)
                {
                    *old = s;
                } else {
                    proposed.snippets.push(s);
                }
                match proposed.validate() {
                    Ok(()) => {
                        self.settings = proposed;
                        if !self.persist() {
                            return;
                        }
                        self.refresh_dictionary();
                        self.status(
                            "Snippet saved; enable explicit commands to speak its trigger.",
                        );
                    }
                    Err(e) => self.status(&e),
                }
            }
            311 => {
                let i = SendMessageW(self.h(43), LB_GETCURSEL, 0, 0);
                if i >= 0 && (i as usize) < self.settings.snippets.len() {
                    self.settings.snippets.remove(i as usize);
                    if !self.persist() {
                        return;
                    }
                    self.refresh_dictionary();
                }
            }
            400 => {
                let i = SendMessageW(self.h(60), LB_GETCURSEL, 0, 0);
                if i >= 0 {
                    match storage::history(&self.dir, self.settings.retention_days) {
                        Ok(items) => {
                            if let Some(item) = items.get(i as usize) {
                                self.set(61, &item.text);
                            }
                        }
                        Err(e) => self.status(&e),
                    }
                }
            }
            401 => {
                if let Err(e) = insertion::copy(self.window, &self.text(61)) {
                    self.status(&e);
                }
            }
            402 => {
                if let Some(p) = file_dialog(self.window, true, "Text\0*.txt\0\0", "txt") {
                    if let Err(e) = storage::atomic_write(&p, self.text(61).as_bytes()) {
                        self.status(&e);
                    }
                }
            }
            404 => {
                if MessageBoxW(
                    self.window,
                    wide("Delete all local transcript history? This cannot be undone.").as_ptr(),
                    wide("VoiceToText").as_ptr(),
                    MB_YESNO | MB_ICONQUESTION,
                ) == IDYES
                {
                    match storage::delete_history(&self.dir) {
                        Ok(()) => {
                            self.refresh_history();
                            self.set(61, "");
                            self.status("History deleted. OS backups / filesystem snapshots are outside this app.");
                        }
                        Err(e) => self.status(&e),
                    }
                }
            }
            _ => {}
        }
    }
    unsafe fn export(&mut self, id: i32) {
        let (filter, ext) = match id {
            106 => ("Markdown\0*.md\0\0", "md"),
            107 => ("SubRip\0*.srt\0\0", "srt"),
            _ => ("Text\0*.txt\0\0", "txt"),
        };
        let content = if id == 107 {
            if !self.transcript.timing_reliable {
                self.status(
                    "Segment times exceed audio bounds or are unreliable. Use TXT/Markdown.",
                );
                return;
            }
            match engine::srt(&self.transcript.segments) {
                Ok(s) => s,
                Err(e) => {
                    self.status(&e);
                    return;
                }
            }
        } else {
            self.text(12)
        };
        if content.is_empty() {
            self.status("No result to export");
            return;
        }
        if let Some(p) = file_dialog(self.window, true, filter, ext) {
            match storage::atomic_write(&p,content.as_bytes()){Ok(())=>self.status(if id==107{"Raw model segments exported. SRT does not include result edits or dictionary formatting."}else{"Result exported locally."}),Err(e)=>self.status(&e)}
        }
    }
}
unsafe extern "system" fn overlay_proc(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            let text = wide(&window_text(hwnd));
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            rect.left += 12;
            rect.right -= 12;
            SetBkMode(dc, TRANSPARENT as i32);
            SetTextColor(dc, 0x002b2830);
            DrawTextW(
                dc,
                text.as_ptr(),
                -1,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            EndPaint(hwnd, &ps);
            0
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize {
    // Do not borrow App on synchronous child notifications. Native controls own accessibility.
    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return 0;
    }
    if msg == WM_COMMAND && (w >> 16) != 0 {
        return DefWindowProcW(hwnd, msg, w, l);
    }
    if [WM_CTLCOLORSTATIC, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX].contains(&msg) {
        let dc = w as HDC;
        let dark = DARK.load(Ordering::Relaxed);
        SetTextColor(dc, if dark { 0x00f5f3f1 } else { 0x00332b24 });
        SetBkColor(dc, if dark { 0x00262120 } else { 0x00ffffff });
        let b = BACKGROUND.load(Ordering::Relaxed);
        if b != 0 {
            return b as isize;
        }
    }
    if msg == WM_ERASEBKGND {
        let b = BACKGROUND.load(Ordering::Relaxed);
        if b != 0 {
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            FillRect(w as HDC, &rect, b as HBRUSH);
            return 1;
        }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, w, l);
    }
    // File pickers and MessageBox run nested message loops. Never create another
    // mutable App borrow while an outer callback is holding one.
    let Some(_guard) = CallbackGuard::enter() else {
        if matches!(
            msg,
            WM_COMMAND
                | WM_HOTKEY
                | WM_TIMER
                | WM_CLOSE
                | WM_DPICHANGED
                | WM_SETTINGCHANGE
                | WM_POWERBROADCAST
                | WM_WTSSESSION_CHANGE
                | TRAY_MSG
        ) {
            return 0;
        }
        return DefWindowProcW(hwnd, msg, w, l);
    };
    match msg {
        WM_COMMAND => {
            (*ptr).action((w & 0xffff) as i32);
            0
        }
        WM_HOTKEY => {
            if w == CANCELKEY as usize {
                (*ptr).cancel();
            } else {
                (*ptr).hotkey();
            }
            0
        }
        WM_TIMER => {
            (*ptr).tick();
            0
        }
        WM_DPICHANGED => {
            (*ptr).dpi((w & 0xffff) as u32, l as *const RECT);
            0
        }
        WM_SETTINGCHANGE => {
            (*ptr).theme();
            0
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_POWERBROADCAST => {
            if w == 4 {
                (*ptr).cancel();
            }
            1
        }
        WM_WTSSESSION_CHANGE => {
            if w == 7 || w == 6 {
                (*ptr).cancel();
            }
            0
        }
        TRAY_MSG => {
            if l as u32 == WM_LBUTTONDBLCLK {
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
            } else if l as u32 == WM_RBUTTONUP {
                // TrackPopupMenu owns a nested loop. End capture before entering it
                // so lock/suspend cannot strand the microphone during that loop.
                if (*ptr).session.phase == Phase::Listening {
                    (*ptr).cancel();
                }
                let menu = CreatePopupMenu();
                AppendMenuW(menu, MF_STRING, 1, wide("Open VoiceToText").as_ptr());
                AppendMenuW(menu, MF_STRING, 2, wide("Start / stop dictation").as_ptr());
                AppendMenuW(menu, MF_STRING, 3, wide("Cancel").as_ptr());
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                AppendMenuW(menu, MF_STRING, 4, wide("Exit — stop microphone").as_ptr());
                let mut point: POINT = std::mem::zeroed();
                GetCursorPos(&mut point);
                SetForegroundWindow(hwnd);
                let id = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_RIGHTBUTTON,
                    point.x,
                    point.y,
                    0,
                    hwnd,
                    null(),
                );
                DestroyMenu(menu);
                match id {
                    1 => {
                        ShowWindow(hwnd, SW_SHOW);
                    }
                    2 => (*ptr).hotkey(),
                    3 => (*ptr).cancel(),
                    4 => {
                        (*ptr).cancel();
                        Shell_NotifyIconW(NIM_DELETE, &(*ptr).tray);
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}
pub fn run() -> Result<(), String> {
    unsafe {
        let name = wide("Local\\VoiceToText-single-instance");
        let mutex = CreateMutexW(null(), 0, name.as_ptr());
        if mutex.is_null() {
            return Err("Could not initialize local instance guard".into());
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(mutex);
            return Err("VoiceToText is already running. Open it from the system tray.".into());
        }
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance = GetModuleHandleW(null());
        let class = wide("VoiceToText-native");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(proc);
        wc.hInstance = instance;
        wc.lpszClassName = class.as_ptr();
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        wc.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("VoiceToText — Offline Bangla Dictation").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            825,
            795,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            CloseHandle(mutex);
            return Err("Could not create native window".into());
        }
        let oclass = wide("VoiceToText-recording");
        wc.lpfnWndProc = Some(overlay_proc);
        wc.lpszClassName = oclass.as_ptr();
        RegisterClassW(&wc);
        let overlay = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            oclass.as_ptr(),
            wide("শুনছি…").as_ptr(),
            WS_POPUP | WS_BORDER,
            30,
            30,
            430,
            55,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        let dpi = GetDpiForWindow(hwnd).max(96);
        SetWindowPos(
            hwnd,
            null_mut(),
            0,
            0,
            825 * dpi as i32 / 96,
            795 * dpi as i32 / 96,
            SWP_NOMOVE | SWP_NOZORDER,
        );
        let font = CreateFontW(
            -17 * dpi as i32 / 96,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide("Nirmala UI").as_ptr(),
        );
        let heading = CreateFontW(
            -27 * dpi as i32 / 96,
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide("Segoe UI").as_ptr(),
        );
        let dir = storage::data_dir();
        let mut settings = storage::read_settings(&dir)?;
        if settings.model_path.is_empty() {
            if let Ok(exe) = std::env::current_exe() {
                let p = exe
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join("models/ggml-small-q5_1.bin");
                if p.exists() {
                    settings.model_path = p.to_string_lossy().into_owned();
                }
            }
        }
        let mut tray: NOTIFYICONDATAW = std::mem::zeroed();
        tray.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        tray.hWnd = hwnd;
        tray.uID = 1;
        tray.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        tray.uCallbackMessage = TRAY_MSG;
        tray.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
        let tip = wide("VoiceToText · Offline dictation");
        tray.szTip[..tip.len()].copy_from_slice(&tip);
        if Shell_NotifyIconW(NIM_ADD, &tray) == 0 {
            DestroyWindow(hwnd);
            CloseHandle(mutex);
            return Err("System tray unavailable".into());
        }
        let now = Instant::now();
        let mut app = Box::new(App {
            window: hwnd,
            overlay,
            font,
            heading,
            controls: BTreeMap::new(),
            page: 0,
            settings,
            dir,
            session: Session::default(),
            capture: None,
            hold_active: false,
            worker: Worker::new(),
            target: None,
            undo: None,
            undo_pending: false,
            confirm_at: None,
            transcript: Transcript::default(),
            last_audio: Vec::new(),
            last_wav: None,
            previous_result: String::new(),
            started: now,
            last_preview: now,
            last_activity: now,
            pending: None,
            armed: false,
            armed_undo: false,
            delivery: None,
            tray,
            devices: capture::devices(),
            status: String::new(),
        });
        app.layout();
        app.theme();
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *app as *mut App) as isize);
        if RegisterHotKey(
            hwnd,
            HOTKEY,
            app.settings.modifiers | MOD_NOREPEAT,
            app.settings.hotkey,
        ) == 0
        {
            app.status("Shortcut conflict. Choose another key in Preferences.");
        }
        if app.settings.model_path.is_empty() {
            app.status("Model প্রয়োজন। Preferences থেকে local multilingual .bin নির্বাচন করুন।");
        } else {
            app.status("প্রস্তুত · Ctrl+Alt+Space default shortcut · Result review is on");
        }
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
        SetTimer(hwnd, 1, 100, None);
        if !std::env::args().any(|a| a == "--hidden") {
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        app.cancel();
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        KillTimer(hwnd, 1);
        UnregisterHotKey(hwnd, HOTKEY);
        UnregisterHotKey(hwnd, CANCELKEY);
        WTSUnRegisterSessionNotification(hwnd);
        Shell_NotifyIconW(NIM_DELETE, &app.tray);
        DestroyWindow(overlay);
        DestroyWindow(hwnd);
        DeleteObject(app.font);
        DeleteObject(app.heading);
        let b = BACKGROUND.swap(0, Ordering::Relaxed);
        if b != 0 {
            DeleteObject(b as HBRUSH);
        }
        CloseHandle(mutex);
        Ok(())
    }
}
