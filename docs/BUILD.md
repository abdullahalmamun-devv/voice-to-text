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

## Windows MSVC recipe — supplied, not verified

Preinstall Rust 1.85.1 with the MSVC x64 target, Visual Studio C++ build tools/Windows SDK, CMake and LLVM/libclang. Use a Visual Studio x64 Developer PowerShell, set `LIBCLANG_PATH` if needed, then:

```powershell
.\scripts\build-windows.ps1
```

`-FastAvx2` is an opt-in hardware-specific build, not the distributed binary. Validate emitted dependencies/instruction requirements separately. This MSVC route has not been executed on Windows in this environment.

## Validation tools

```sh
python3 scripts/smoke-worker.py target/release/voicetotext models/ggml-small-q5_1.bin local-16k-mono.wav smoke-results
python3 scripts/benchmark.py target/release/voicetotext models/ggml-small-q5_1.bin corpus.csv benchmark-results
```

Corpus columns: `id,audio,reference,slice,language`; audio paths relative to CSV, language bn/en/auto. Use consenting/rights-cleared recordings and keep speakers separate between development/holdout sets. Benchmark output intentionally persists test references/transcripts. Raw/normalized WER and scalar CER are separate; critical entities/correction effort and speaker allocation must be scored separately under the PRD.

For offline rebuilds, preinstall the toolchain/target components before disconnecting; rustup cannot fetch missing components offline. Vendoring eliminates Cargo crate downloads, not the need for compilers. The source and packages are not guaranteed byte-identical across compiler/OS versions.
