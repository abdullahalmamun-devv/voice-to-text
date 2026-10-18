param([switch]$FastAvx2)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Install Rust 1.85.1 before building.' }
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) { throw 'Install CMake before building.' }
# Run from a Visual Studio x64 Developer PowerShell with LLVM/libclang installed.
# Dependencies are vendored; cargo does not need network access.
$env:GGML_NATIVE = 'OFF'
$env:GGML_OPENMP = 'OFF'
$env:GGML_AVX = $(if ($FastAvx2) { 'ON' } else { 'OFF' })
$env:GGML_AVX2 = $env:GGML_AVX
$env:GGML_F16C = $env:GGML_AVX
$env:GGML_FMA = $env:GGML_AVX
cargo build --offline --locked --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
Write-Host 'Built target\x86_64-pc-windows-msvc\release\voicetotext.exe'
