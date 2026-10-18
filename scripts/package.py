#!/usr/bin/env python3
"""Development packaging: local binary + model; no downloads. Python is not a user runtime.
Usage: package.py WINDOWS.exe LINUX_BINARY OUTPUT_DIRECTORY
"""
import hashlib
import json
from pathlib import Path
import shutil
import sys
import zipfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = "0.2.0-alpha.1"


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for data in iter(lambda: f.read(1024 * 1024), b""):
            h.update(data)
    return h.hexdigest()


def archive(folder, target):
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for f in sorted(folder.rglob("*")):
            if f.is_file():
                z.write(f, folder.name + "/" + str(f.relative_to(folder)))


def main():
    exe, linux, out = map(lambda x: Path(x).resolve(), sys.argv[1:4])
    assert exe.is_file() and linux.is_file()
    model = ROOT / "models/ggml-small-q5_1.bin"
    m = json.loads((ROOT / "models/ggml-small-q5_1.manifest.json").read_text())
    assert model.stat().st_size == m["size_bytes"] and digest(model) == m["sha256"]
    out.mkdir(parents=True, exist_ok=True)
    win = out / ("VoiceToText-Windows-" + VERSION)
    source = out / ("VoiceToText-Source-" + VERSION)
    for folder in (win, source):
        if folder.exists():
            shutil.rmtree(folder)
        folder.mkdir()
        for name in ("README.md", "START-HERE.md", "LICENSE"):
            shutil.copy2(ROOT / name, folder / name)
        for name in ("docs", "licenses"):
            shutil.copytree(ROOT / name, folder / name)
    shutil.copy2(exe, win / "VoiceToText.exe")
    shutil.copytree(ROOT / "models", win / "models")
    (win / "scripts").mkdir()
    for name in ("Install.ps1", "Verify-Package.ps1"):
        shutil.copy2(ROOT / "scripts" / name, win / "scripts" / name)
    for name, script in (("Install.cmd", "Install.ps1"), ("Verify.cmd", "Verify-Package.ps1")):
        (win / name).write_bytes((f'@echo off\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\\{script}"\r\npause\r\n').encode("ascii"))
    files = [{"path": str(f.relative_to(win)).replace("\\", "/"), "size_bytes": f.stat().st_size,
              "sha256": digest(f)} for f in sorted(win.rglob("*")) if f.is_file()]
    (win / "package-manifest.json").write_text(json.dumps({"version": VERSION, "files": files}, indent=2))
    for name in ("src", "vendor", ".cargo", "assets", "scripts"):
        shutil.copytree(ROOT / name, source / name, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
    for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "build.rs"):
        shutil.copy2(ROOT / name, source / name)
    (source / "models").mkdir()
    shutil.copy2(ROOT / "models/ggml-small-q5_1.manifest.json", source / "models")
    (source / "bin").mkdir()
    shutil.copy2(linux, source / "bin/voicetotext-linux-x64")
    targets = [out / ("VoiceToText-Windows-Offline-" + VERSION + ".zip"),
               out / ("VoiceToText-Source-" + VERSION + ".zip")]
    archive(win, targets[0])
    archive(source, targets[1])
    shutil.copy2(ROOT / "START-HERE.md", out / "START-HERE.md")
    (out / "SHA256SUMS.txt").write_text("".join(f"{digest(f)}  {f.name}\n" for f in targets + [out / "START-HERE.md"]))
    print(json.dumps([{"file": f.name, "bytes": f.stat().st_size, "sha256": digest(f)} for f in targets]))


if __name__ == "__main__":
    main()
