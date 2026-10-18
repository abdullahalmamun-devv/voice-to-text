# Local dependency patches

Cargo.toml uses `[patch.crates-io]` path dependencies so the patches are actually compiled. Lockfile versions remain whisper-rs 0.15.1 and whisper-rs-sys 0.14.1. Corresponding full modified sources are in `vendor/`.

| File | Change | Reason |
|---|---|---|
| whisper-rs/src/whisper_params.rs | Own language, initial prompt and VAD-path C strings with clone-safe `Option<Arc<CString>>`; keep raw pointers valid while params/clones live | Replace upstream `into_raw()` allocations that were never reclaimed; bounded repeated inference memory |
| whisper-rs-sys/build.rs | On GNU Windows link libstdc++/winpthread statically and discover compiler library search directories | Portable offline package does not require compiler-runtime DLL installation |
| whisper-rs-sys/build.rs | Copy cross-built `whisper.a`/`ggml*.a` archive names to conventional `lib*.a` aliases before Rust linking | Align MinGW/CMake archive naming in the tested cross toolchain |

The speech decoder/weights are not fine-tuned and no accuracy patch is claimed. Vendored third-party license texts remain intact. Remaining upstream build warnings about unused imports/mutability are not product errors.

App-level adapter: GNU Windows reads model bytes via Rust and initializes context from a buffer, avoiding the upstream narrow-path loader on that target. Non-GNU platforms retain the upstream file loader. `scripts/mingw-compat.h` and the app build script embed/support the Windows SDK manifest without modifying inference.

## Alpha.2 changes

The sys build explicitly forwards GGML_NATIVE/AVX/AVX2/AVX512/FMA/F16C/OPENMP to CMake, defaults OFF, and tracks environment changes. The earlier environment-only portable CPU claim was insufficient. Both model loaders now use Windows UTF-16 paths across toolchains. The patched translation unit is refreshed in the build cache; the GNU adapter's model-sized buffer is removed. FastAvx2 builds require separate compatible hardware qualification.
