//! One reusable local subprocess. Pipe IPC only; no listening ports or audio files.
use crate::engine::{Engine, Transcript};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
};
#[derive(Serialize, Deserialize)]
pub struct Request {
    pub session_id: u64,
    pub model: String,
    pub samples: Vec<f32>,
    pub language: String,
    pub prompt: String,
    pub preview: bool,
}
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub session_id: u64,
    pub preview: bool,
    pub transcript: Option<Transcript>,
    pub error: Option<String>,
}
pub fn serve() -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = std::io::stdout().lock();
    let mut engine: Option<Engine> = None;
    let mut loaded = String::new();
    loop {
        let mut bytes = Vec::new();
        let mut limited = (&mut reader).take(160_000_001);
        let n = limited
            .read_until(b'\n', &mut bytes)
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if n > 160_000_000 || bytes.last() != Some(&b'\n') {
            return Err("IPC message exceeds limit".into());
        }
        let req: Request =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid local IPC message")?;
        let result = (|| {
            if loaded != req.model {
                engine = None;
                loaded.clear();
                let fresh = Engine::load(Path::new(&req.model))?;
                engine = Some(fresh);
                loaded = req.model.clone();
            }
            engine.as_ref().ok_or("Model unavailable")?.transcribe(
                &req.samples,
                &req.language,
                &req.prompt,
                req.preview,
            )
        })();
        let response = match result {
            Ok(t) => Response {
                session_id: req.session_id,
                preview: req.preview,
                transcript: Some(t),
                error: None,
            },
            Err(e) => Response {
                session_id: req.session_id,
                preview: req.preview,
                transcript: None,
                error: Some(e),
            },
        };
        serde_json::to_writer(&mut stdout, &response).map_err(|e| e.to_string())?;
        stdout.write_all(b"\n").map_err(|e| e.to_string())?;
        stdout.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub struct Worker {
    pub tx: mpsc::SyncSender<Request>,
    pub rx: mpsc::Receiver<Response>,
    child: Arc<Mutex<Option<Child>>>,
    pub busy: Arc<std::sync::atomic::AtomicBool>,
}
impl Worker {
    pub fn new() -> Self {
        let (tx, jobs) = mpsc::sync_channel::<Request>(1);
        let (results, rx) = mpsc::channel();
        let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
        let handle = child.clone();
        let busy = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = busy.clone();
        std::thread::spawn(move || {
            let mut pipes = None;
            for req in jobs {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                let response = (|| -> Result<Response, String> {
                    let mut lock = handle.lock().map_err(|_| "Worker lock failed")?;
                    if lock
                        .as_mut()
                        .and_then(|c| c.try_wait().ok().flatten())
                        .is_some()
                    {
                        *lock = None;
                        pipes = None;
                    }
                    if lock.is_none() {
                        let mut command =
                            Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
                        command
                            .arg("--worker")
                            .stdin(Stdio::piped())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::null());
                        #[cfg(windows)]
                        {
                            use std::os::windows::process::CommandExt;
                            command.creation_flags(0x08000000);
                        }
                        let mut c = command
                            .spawn()
                            .map_err(|_| "Could not start local inference worker")?;
                        let input = c.stdin.take().ok_or("Worker input unavailable")?;
                        let output =
                            BufReader::new(c.stdout.take().ok_or("Worker output unavailable")?);
                        pipes = Some((input, output));
                        *lock = Some(c);
                    }
                    drop(lock);
                    let (input, output) = pipes.as_mut().ok_or("Worker unavailable")?;
                    serde_json::to_writer(&mut *input, &req).map_err(|_| "Worker stopped")?;
                    input.write_all(b"\n").map_err(|_| "Worker stopped")?;
                    input.flush().map_err(|_| "Worker stopped")?;
                    let mut line = String::new();
                    if output.read_line(&mut line).map_err(|_| "Worker stopped")? == 0 {
                        return Err("Worker exited. Retry the recording.".into());
                    }
                    if line.len() > 1_000_000 {
                        return Err("Invalid worker output".into());
                    }
                    serde_json::from_str(&line).map_err(|_| "Invalid worker response".into())
                })();
                let response = response.unwrap_or_else(|e| {
                    pipes = None;
                    // A broken pipe or malformed response invalidates the process too.
                    // Otherwise a subsequent retry could keep an unusable live child.
                    if let Ok(mut lock) = handle.lock() {
                        if let Some(mut c) = lock.take() {
                            let _ = c.kill();
                            let _ = c.wait();
                        }
                    }
                    Response {
                        session_id: req.session_id,
                        preview: req.preview,
                        transcript: None,
                        error: Some(e),
                    }
                });
                let _ = results.send(response);
                flag.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            if let Ok(mut lock) = handle.lock() {
                if let Some(mut c) = lock.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            }
        });
        Self {
            tx,
            rx,
            child,
            busy,
        }
    }
    pub fn cancel(&self) {
        if let Ok(mut lock) = self.child.lock() {
            if let Some(mut c) = lock.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}
impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}
