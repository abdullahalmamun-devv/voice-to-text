//! Minimal, version-pinned sherpa-onnx C ABI. Libraries are verified before load.
//! Opaque native objects stay inside the existing cancellable worker process.
use crate::{engine::Transcript, model_pack};
use libloading::Library;
use std::{
    ffi::{c_char, c_void, CStr, CString},
    path::Path,
    time::Instant,
};

#[repr(C)]
struct Transducer {
    encoder: *const c_char,
    decoder: *const c_char,
    joiner: *const c_char,
}
#[repr(C)]
struct ModelConfig {
    transducer: Transducer,
    paraformer: [*const c_char; 2],
    zipformer2_ctc: *const c_char,
    tokens: *const c_char,
    num_threads: i32,
    provider: *const c_char,
    debug: i32,
    model_type: *const c_char,
    modeling_unit: *const c_char,
    bpe_vocab: *const c_char,
    tokens_buf: *const c_char,
    tokens_buf_size: i32,
    nemo_ctc: *const c_char,
    t_one_ctc: *const c_char,
}
#[repr(C)]
struct FstConfig {
    graph: *const c_char,
    max_active: i32,
}
#[repr(C)]
struct Config {
    feature: [i32; 2],
    model: ModelConfig,
    decoding_method: *const c_char,
    max_active_paths: i32,
    enable_endpoint: i32,
    rule1: f32,
    rule2: f32,
    rule3: f32,
    hotwords_file: *const c_char,
    hotwords_score: f32,
    ctc_fst: FstConfig,
    rule_fsts: *const c_char,
    rule_fars: *const c_char,
    blank_penalty: f32,
    hotwords_buf: *const c_char,
    hotwords_buf_size: i32,
    homophone: [*const c_char; 3],
}
type Handle = *const c_void;
type Create = unsafe extern "C" fn(*const Config) -> Handle;
type Destroy = unsafe extern "C" fn(Handle);
type CreateStream = unsafe extern "C" fn(Handle) -> Handle;
type Accept = unsafe extern "C" fn(Handle, i32, *const f32, i32);
type Ready = unsafe extern "C" fn(Handle, Handle) -> i32;
type Decode = unsafe extern "C" fn(Handle, Handle);
type ResultJson = unsafe extern "C" fn(Handle, Handle) -> *const c_char;
type FreeJson = unsafe extern "C" fn(*const c_char);
type Version = unsafe extern "C" fn() -> *const c_char;
struct Api {
    create: Create,
    destroy: Destroy,
    create_stream: CreateStream,
    destroy_stream: Destroy,
    accept: Accept,
    ready: Ready,
    decode: Decode,
    finish: Destroy,
    result: ResultJson,
    free_result: FreeJson,
    // Declaration order ensures C API unloads before its ONNX dependency.
    _capi: Library,
    _onnx: Library,
}

fn load_library(path: &Path) -> Result<Library, String> {
    // Callers have verified exact compiled-in upstream hashes. Absolute paths and
    // Windows search flags avoid resolving model-selected or working-dir DLLs.
    #[cfg(windows)]
    unsafe {
        libloading::os::windows::Library::load_with_flags(path, 0x00000100 | 0x00000800)
            .map(Library::from)
            .map_err(|_| {
                "Pinned native runtime could not load; check Windows runtime prerequisites".into()
            })
    }
    #[cfg(not(windows))]
    unsafe {
        Library::new(path).map_err(|_| "Pinned native runtime could not load".into())
    }
}
impl Api {
    fn load(pack: &model_pack::Pack) -> Result<Self, String> {
        let root = pack.root.join("runtime");
        let onnx = load_library(&root.join(&pack.runtime.onnx))?;
        let capi = load_library(&root.join(&pack.runtime.capi))?;
        // Symbols are copied function pointers; both libraries remain owned by Api.
        unsafe {
            macro_rules! symbol {
                ($name:literal, $ty:ty) => {
                    *capi
                        .get::<$ty>(concat!($name, "\0").as_bytes())
                        .map_err(|_| "Pinned runtime API mismatch")?
                };
            }
            let version = symbol!("SherpaOnnxGetVersionStr", Version)();
            if version.is_null() || CStr::from_ptr(version).to_bytes() != b"1.13.8" {
                return Err("Unsupported native runtime version".into());
            }
            Ok(Self {
                create: symbol!("SherpaOnnxCreateOnlineRecognizer", Create),
                destroy: symbol!("SherpaOnnxDestroyOnlineRecognizer", Destroy),
                create_stream: symbol!("SherpaOnnxCreateOnlineStream", CreateStream),
                destroy_stream: symbol!("SherpaOnnxDestroyOnlineStream", Destroy),
                accept: symbol!("SherpaOnnxOnlineStreamAcceptWaveform", Accept),
                ready: symbol!("SherpaOnnxIsOnlineStreamReady", Ready),
                decode: symbol!("SherpaOnnxDecodeOnlineStream", Decode),
                finish: symbol!("SherpaOnnxOnlineStreamInputFinished", Destroy),
                result: symbol!("SherpaOnnxGetOnlineStreamResultAsJson", ResultJson),
                free_result: symbol!("SherpaOnnxDestroyOnlineStreamResultJson", FreeJson),
                _capi: capi,
                _onnx: onnx,
            })
        }
    }
}
pub(crate) struct Zipformer {
    api: Api,
    recognizer: Handle,
    _paths: Vec<CString>,
}
impl Zipformer {
    pub fn load(path: &Path) -> Result<Self, String> {
        let pack = model_pack::verify(path)?;
        let api = Api::load(&pack)?;
        let paths: Vec<CString> = ["encoder.onnx", "decoder.onnx", "joiner.onnx", "tokens.txt"]
            .iter()
            .map(|name| {
                CString::new(
                    pack.root
                        .join(name)
                        .to_str()
                        .ok_or("Model path is not UTF-8")?,
                )
                .map_err(|_| "Invalid model path")
            })
            .collect::<Result<_, _>>()?;
        // The public v1.13.8 header specifies zero-init. All fields are raw
        // pointers, integers, floats or nested repr(C) structures; zero is valid.
        let mut config: Config = unsafe { std::mem::zeroed() };
        config.feature = [16000, 80];
        config.model.transducer = Transducer {
            encoder: paths[0].as_ptr(),
            decoder: paths[1].as_ptr(),
            joiner: paths[2].as_ptr(),
        };
        config.model.tokens = paths[3].as_ptr();
        config.model.num_threads = 2;
        config.model.provider = c"cpu".as_ptr();
        config.model.model_type = c"zipformer2".as_ptr();
        config.model.modeling_unit = c"cjkchar".as_ptr();
        config.decoding_method = c"modified_beam_search".as_ptr();
        config.max_active_paths = 4;
        let recognizer = unsafe { (api.create)(&config) };
        if recognizer.is_null() {
            return Err("Bengali model could not load. Check pack and available memory.".into());
        }
        Ok(Self {
            api,
            recognizer,
            _paths: paths,
        })
    }
    pub fn transcribe(
        &self,
        samples: &[f32],
        language: &str,
        prompt: &str,
        load_ms: u64,
    ) -> Result<Transcript, String> {
        if language != "bn" {
            return Err("This experimental model requires Bengali mode; select a multilingual model for English/Auto".into());
        }
        if !prompt.is_empty() {
            return Err("Vocabulary hints are not supported by this experimental Bengali profile; disable dictionary hints or select a multilingual model".into());
        }
        let start = Instant::now();
        let stream = unsafe { (self.api.create_stream)(self.recognizer) };
        if stream.is_null() {
            return Err("Could not allocate Bengali inference stream".into());
        }
        struct Stream<'a> {
            handle: Handle,
            api: &'a Api,
        }
        impl Drop for Stream<'_> {
            fn drop(&mut self) {
                unsafe { (self.api.destroy_stream)(self.handle) }
            }
        }
        let stream = Stream {
            handle: stream,
            api: &self.api,
        };
        let tail = [0f32; 8000];
        unsafe {
            (self.api.accept)(stream.handle, 16000, samples.as_ptr(), samples.len() as i32);
            (self.api.accept)(stream.handle, 16000, tail.as_ptr(), tail.len() as i32);
            (self.api.finish)(stream.handle);
            while (self.api.ready)(self.recognizer, stream.handle) == 1 {
                (self.api.decode)(self.recognizer, stream.handle);
            }
        }
        let raw = unsafe { (self.api.result)(self.recognizer, stream.handle) };
        if raw.is_null() {
            return Err("Bengali runtime returned no result".into());
        }
        let result = (|| -> Result<String, String> {
            let bytes = unsafe { CStr::from_ptr(raw) }.to_bytes();
            if bytes.len() > 1_000_000 {
                return Err("Invalid native inference response".into());
            }
            let value: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|_| "Invalid native inference text")?;
            Ok(value
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("Native transcript missing")?
                .trim()
                .to_owned())
        })();
        unsafe { (self.api.free_result)(raw) };
        let text = result?;
        Ok(Transcript {
            review_required: crate::text::unexpected_script(&text, "bn"),
            text,
            decode_ms: start.elapsed().as_millis() as u64,
            load_ms,
            recognized_language: Some("bn".into()),
            no_timestamps: true,
            experimental_model: true,
            ..Transcript::default()
        })
    }
}
impl Drop for Zipformer {
    fn drop(&mut self) {
        unsafe { (self.api.destroy)(self.recognizer) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_c_abi_layout() {
        // Measured independently by native/check-abi.c against upstream v1.13.8.
        assert_eq!(std::mem::size_of::<Config>(), 272);
        assert_eq!(std::mem::size_of::<ModelConfig>(), 136);
        assert_eq!(std::mem::size_of::<FstConfig>(), 16);
        assert_eq!(std::mem::offset_of!(Config, model), 8);
        assert_eq!(std::mem::offset_of!(Config, decoding_method), 144);
        assert_eq!(std::mem::offset_of!(Config, hotwords_file), 176);
        assert_eq!(std::mem::offset_of!(Config, ctc_fst), 192);
        assert_eq!(std::mem::offset_of!(Config, hotwords_buf), 232);
        assert_eq!(std::mem::offset_of!(Config, homophone), 248);
        assert_eq!(std::mem::offset_of!(ModelConfig, provider), 64);
        assert_eq!(std::mem::offset_of!(ModelConfig, t_one_ctc), 128);
    }
}
