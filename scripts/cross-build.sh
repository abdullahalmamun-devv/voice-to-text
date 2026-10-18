#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${MINGW_ROOT:?Set MINGW_ROOT to the compiler prefix containing usr/bin}"
: "${LIBCLANG_PATH:?Set LIBCLANG_PATH to a directory containing libclang}"
export PATH="$MINGW_ROOT/usr/bin:$PATH"
export LD_LIBRARY_PATH="$LIBCLANG_PATH${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export BINDGEN_EXTRA_CLANG_ARGS="--target=x86_64-w64-windows-gnu -I$MINGW_ROOT/usr/x86_64-w64-mingw32/include -I$MINGW_ROOT/usr/lib/gcc/x86_64-w64-mingw32/13-posix/include"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export CFLAGS_x86_64_pc_windows_gnu="-include $PWD/scripts/mingw-compat.h"
export CXXFLAGS_x86_64_pc_windows_gnu="$CFLAGS_x86_64_pc_windows_gnu"
export GGML_NATIVE=OFF GGML_OPENMP=OFF
cargo build --offline --locked --release --target x86_64-pc-windows-gnu
