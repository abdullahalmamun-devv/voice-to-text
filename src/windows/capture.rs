use std::{
    mem::size_of,
    ptr::null_mut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use windows_sys::Win32::Media::Audio::*;
const MM_WIM_DATA: u32 = 0x03c0;
use voicetotext::audio::MAX_TURN;
struct Buffer {
    header: Box<WAVEHDR>,
    _data: Box<[i16]>,
}
struct State {
    samples: Mutex<Vec<f32>>,
    active: AtomicBool,
}
pub struct Capture {
    handle: HWAVEIN,
    state: Box<State>,
    buffers: Vec<Buffer>,
}
unsafe extern "system" fn callback(
    h: HWAVEIN,
    message: u32,
    instance: usize,
    param: usize,
    _: usize,
) {
    if message != MM_WIM_DATA || instance == 0 || param == 0 {
        return;
    }
    let s = &*(instance as *const State);
    let hdr = &mut *(param as *mut WAVEHDR);
    if s.active.load(Ordering::SeqCst) {
        let data =
            std::slice::from_raw_parts(hdr.lpData as *const i16, hdr.dwBytesRecorded as usize / 2);
        if let Ok(mut samples) = s.samples.lock() {
            let room = MAX_TURN.saturating_sub(samples.len());
            samples.extend(data.iter().take(room).map(|x| *x as f32 / 32768.));
        }
        if s.active.load(Ordering::SeqCst) {
            let _ = waveInAddBuffer(h, hdr, size_of::<WAVEHDR>() as u32);
        }
    }
}
pub fn devices() -> Vec<(u32, String)> {
    let mut v = vec![(u32::MAX, "System default microphone".into())];
    unsafe {
        for id in 0..waveInGetNumDevs() {
            let mut caps: WAVEINCAPSW = std::mem::zeroed();
            if waveInGetDevCapsW(id as usize, &mut caps, size_of::<WAVEINCAPSW>() as u32) == 0 {
                let name = caps.szPname;
                let end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
                v.push((id, String::from_utf16_lossy(&name[..end])));
            }
        }
    }
    v
}
impl Capture {
    pub fn start(device: u32) -> Result<Self, String> {
        unsafe {
            let state = Box::new(State {
                samples: Mutex::new(Vec::with_capacity(16000 * 15)),
                active: AtomicBool::new(true),
            });
            let mut handle = null_mut();
            let format = WAVEFORMATEX {
                wFormatTag: 1,
                nChannels: 1,
                nSamplesPerSec: 16000,
                nAvgBytesPerSec: 32000,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            let code = waveInOpen(
                &mut handle,
                device,
                &format,
                callback as usize,
                (&*state as *const State) as usize,
                CALLBACK_FUNCTION,
            );
            if code != 0 {
                return Err(format!("Microphone unavailable (code {code}). Check Windows microphone privacy settings or choose another device."));
            }
            let mut c = Self {
                handle,
                state,
                buffers: Vec::new(),
            };
            for _ in 0..4 {
                let mut data = vec![0i16; 1024].into_boxed_slice();
                let mut header: Box<WAVEHDR> = Box::new(std::mem::zeroed());
                header.lpData = data.as_mut_ptr() as *mut u8;
                header.dwBufferLength = (data.len() * 2) as u32;
                if waveInPrepareHeader(handle, &mut *header, size_of::<WAVEHDR>() as u32) != 0 {
                    return Err("Microphone buffer preparation failed".into());
                }
                c.buffers.push(Buffer {
                    header,
                    _data: data,
                });
                let last = c
                    .buffers
                    .last_mut()
                    .ok_or("Microphone buffer unavailable")?;
                if waveInAddBuffer(handle, &mut *last.header, size_of::<WAVEHDR>() as u32) != 0 {
                    return Err("Microphone buffer failed".into());
                }
            }
            if waveInStart(handle) != 0 {
                return Err("Microphone could not start".into());
            }
            Ok(c)
        }
    }
    pub fn samples(&self) -> Vec<f32> {
        self.state
            .samples
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }
    pub fn len(&self) -> usize {
        self.state.samples.lock().map(|s| s.len()).unwrap_or(0)
    }
    pub fn level(&self) -> f32 {
        self.state
            .samples
            .lock()
            .map(|s| {
                let part = &s[s.len().saturating_sub(1600)..];
                if part.is_empty() {
                    0.
                } else {
                    (part.iter().map(|x| x * x).sum::<f32>() / part.len() as f32).sqrt()
                }
            })
            .unwrap_or(0.)
    }
    pub fn stop(mut self) -> Vec<f32> {
        self.close();
        self.samples()
    }
    fn close(&mut self) {
        unsafe {
            if self.handle.is_null() {
                return;
            }
            self.state.active.store(false, Ordering::SeqCst);
            waveInStop(self.handle);
            waveInReset(self.handle);
            for b in &mut self.buffers {
                waveInUnprepareHeader(self.handle, &mut *b.header, size_of::<WAVEHDR>() as u32);
            }
            if waveInClose(self.handle) != 0 {
                // Keep callback allocations alive if the OS failed to release the handle.
                // This is a fault path, never free memory still referenced by the driver.
                let state = std::mem::replace(
                    &mut self.state,
                    Box::new(State {
                        samples: Mutex::new(Vec::new()),
                        active: AtomicBool::new(false),
                    }),
                );
                let _ = Box::leak(state);
                for b in self.buffers.drain(..) {
                    std::mem::forget(b);
                }
            }
            self.handle = null_mut();
        }
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.close();
    }
}
