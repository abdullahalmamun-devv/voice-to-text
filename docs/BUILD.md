# Offline source build

Source dependencies are in `vendor/`; Cargo is locked and configured to use them. Local path patches apply to whisper-rs and whisper-rs-sys (see DEPENDENCY-PATCHES.md). The model weights and toolchains are not duplicated in the source archive. Copy the Windows package's `models/` for local development. `GGML_NATIVE=OFF` is portable CPU mode; no GPU backend is enabled.

## Tested Linux CLI/core recipe

Preinstall Rust 1.85.1, CMake, a C/C++ compiler and normal platform development libraries. From the source root:

```sh
export WHISPER_DONT_GENERATE_BINDINGS=1
export GGML_NATIVE=OFF GGML_OPENMP=OFF
cargo +1.85.1 test --offline --locked --lib
cargo +1.85.1 build --offline --locked --release
cargo +1.85.1 clippy --offline --locked --all-targets --no-deps -- -D warnings
./target/release/voicetotext --transcribe models/ggml-small-q5_1.bin sample.wav result.json bn
```

This compiles no Linux desktop UI. The prebuilt Linux CLI depends on system glibc/libstdc++; compatibility with arbitrary Linux distributions is not claimed. Python is used only for development benchmark/package scripts.

## Tested GNU Windows cross-build

Preinstall Rust 1.85.1 with `x86_64-pc-windows-gnu`, CMake, MinGW-w64 GCC/G++ 13-posix and libclang. The script expects a compiler prefix with `usr/bin`, `usr/x86_64-w64-mingw32/include` and `usr/lib/gcc/x86_64-w64-mingw32/13-posix/include`; adapt these paths for another distribution/version. In this handoff compiler packages were unpacked locally under such a prefix, not installed system-wide.

```sh
export MINGW_ROOT=/absolute/path/to/mingw-prefix
export LIBCLANG_PATH=/absolute/path/to/libclang-directory
bash scripts/cross-build.sh
```

Output: `target/x86_64-pc-windows-gnu/release/voicetotext.exe`. Rename to `VoiceToText.exe` when packaging. C++ and pthread runtime libraries are statically linked by the documented sys-crate patch. Windows system DLLs remain OS dependencies. `scripts/mingw-compat.h` supplies power-throttling declarations absent in the SDK used here. The build embeds the native DPI/common-controls manifest.

## Windows MSVC build

Preinstall Rust 1.85.1 with the MSVC x64 target, Visual Studio C++ build tools/Windows SDK, CMake and LLVM/libclang. Use a Visual Studio x64 Developer PowerShell, set `LIBCLANG_PATH` if needed, then:

```powershell
.\scripts\build-windows.ps1
```

`-FastAvx2` is an opt-in hardware-specific build, not the distributed binary. Validate emitted dependencies/instruction requirements separately. GitHub Actions executes the MSVC x64 compiler/Windows SDK route: ABI probe, library tests, GUI test-target compilation and release build. The wrapper script above is a local convenience recipe; it is not itself the CI command. Windows GUI/hardware and clean-machine acceptance remain separate.

## Validation tools

```sh
python3 scripts/smoke-worker.py target/release/voicetotext models/ggml-small-q5_1.bin local-16k-mono.wav smoke-results
python3 scripts/benchmark.py target/release/voicetotext models/ggml-small-q5_1.bin corpus.csv benchmark-results
```

Corpus columns: `id,audio,reference,slice,language`; audio paths relative to CSV, language bn/en/auto. Use consenting/rights-cleared recordings and keep speakers separate between development/holdout sets. Benchmark output intentionally persists test references/transcripts. Raw/normalized WER and scalar CER are separate; critical entities/correction effort and speaker allocation must be scored separately under the PRD.

For offline rebuilds, preinstall the toolchain/target components before disconnecting; rustup cannot fetch missing components offline. Vendoring eliminates Cargo crate downloads, not the need for compilers. The source and packages are not guaranteed byte-identical across compiler/OS versions.

## Experimental native Bengali pack

The app does not download models. For a connected **developer/CI setup only**:

```sh
python3 scripts/fetch-bengali-test-pack.py --platform linux-x64 --output model-packs/bengali
python3 scripts/fetch-bengali-corpus.py model-packs/fleurs-bn-30
python3 scripts/smoke-native-bengali.py target/release/voicetotext model-packs/bengali/bengali.vtt-model.json model-packs/bengali-downloads/test-wavs native-verification/smoke
python3 scripts/benchmark-worker.py target/release/voicetotext model-packs/bengali/bengali.vtt-model.json model-packs/fleurs-bn-30/corpus.csv native-verification/fleurs --cpu-profile "record actual host and network context" --no-timestamps
python3 scripts/verify-native-bengali-results.py native-verification/fleurs
```

On Windows use `--platform windows-x64`, `python` and the `.exe` binary.
The setup helper verifies fixed model and upstream release hashes. With already
available local files, `prepare-bengali-pack.py --help` gives the offline preparation
recipe. The app itself requires no Python. It verifies compiled-in model/runtime
pins before native loading, rejects other runtime files and requires Bengali mode.
No segment-timing/SRT, English/Auto or vocabulary-hint support is claimed for this pack.

The workflow tests Linux inference in a disconnected network namespace and Windows
inference with an explicit outbound program rule; these are not a full Windows
traffic-capture audit. See [EXPERIMENTAL-BENGALI.md](EXPERIMENTAL-BENGALI.md).
