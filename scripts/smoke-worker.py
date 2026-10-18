#!/usr/bin/env python3
"""Development-only pipe-worker smoke test. Requires a local 16 kHz mono PCM WAV.
No network requests. Linux /proc observations are not Windows GUI QA.
"""
import argparse
import json
import os
from pathlib import Path
import struct
import subprocess
import threading
import time
import wave


def main():
    p = argparse.ArgumentParser()
    p.add_argument("binary", type=Path)
    p.add_argument("model", type=Path)
    p.add_argument("wav", type=Path)
    p.add_argument("output", type=Path)
    a = p.parse_args()
    binary, model = str(a.binary.resolve()), str(a.model.resolve())
    with wave.open(str(a.wav), "rb") as w:
        assert (w.getframerate(), w.getnchannels(), w.getsampwidth()) == (16000, 1, 2)
        raw = w.readframes(w.getnframes())
        speech = [x / 32768 for x in struct.unpack("<%dh" % (len(raw) // 2), raw)]
    a.output.mkdir(parents=True, exist_ok=True)
    cwd = a.output / "worker-working-directory"
    cwd.mkdir(exist_ok=True)
    observations = {"peak_rss_kib": 0, "rss_samples": 0, "socket_descriptors_seen": [],
                    "max_fd_count": 0, "descriptor_inspection_accessible": True}
    stop = threading.Event()
    active = [None]

    def monitor():
        while not stop.wait(0.05):
            child = active[0]
            if child is None:
                continue
            try:
                for line in Path(f"/proc/{child.pid}/status").read_text().splitlines():
                    if line.startswith("VmRSS:"):
                        observations["rss_samples"] += 1
                        observations["peak_rss_kib"] = max(observations["peak_rss_kib"], int(line.split()[1]))
                fds = list(Path(f"/proc/{child.pid}/fd").iterdir())
                observations["max_fd_count"] = max(observations["max_fd_count"], len(fds))
                for fd in fds:
                    link = os.readlink(fd)
                    if link.startswith("socket:") and link not in observations["socket_descriptors_seen"]:
                        observations["socket_descriptors_seen"].append(link)
            except PermissionError:
                observations["descriptor_inspection_accessible"] = False
            except (FileNotFoundError, ProcessLookupError):
                pass

    def start():
        c = subprocess.Popen([binary, "--worker"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=subprocess.DEVNULL, cwd=cwd, text=True)
        active[0] = c
        return c

    def send(c, ident, samples, preview=False, language="en", prompt=""):
        request = dict(session_id=ident, model=model, samples=samples, language=language,
                       prompt=prompt, preview=preview)
        c.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        c.stdin.flush()

    def exchange(c, ident, samples, **kwargs):
        began = time.perf_counter()
        send(c, ident, samples, **kwargs)
        result = json.loads(c.stdout.readline())
        assert result["session_id"] == ident
        return result, round((time.perf_counter() - began) * 1000)

    thread = threading.Thread(target=monitor, daemon=True)
    thread.start()
    c = start()
    results = []
    try:
        for i in range(500):
            r, elapsed = exchange(c, i + 1, [0.0] * 16000)
            assert not r["error"] and r["transcript"]["text"] == ""
            if i in (0, 499):
                results.append(dict(case=f"silence-{i+1}", wall_ms=elapsed, result=r))
        for i in range(6):
            preview = i % 3 == 0
            r, elapsed = exchange(c, 1001 + i, speech, preview=preview, prompt="English speech")
            assert not r["error"] and r["transcript"]["text"]
            results.append(dict(case="speech-preview" if preview else "speech-final", wall_ms=elapsed, result=r))
        r, _ = exchange(c, 2001, speech, prompt="unsafe\0prompt")
        assert r["error"] == "Invalid vocabulary prompt"
        r, _ = exchange(c, 2002, [0.0] * 16000)
        assert not r["error"] and not r["transcript"]["text"]
        send(c, 3001, speech)
        time.sleep(0.15)
        c.kill()
        c.wait(timeout=10)
        c.stdin.close()
        c.stdout.close()
        c = start()
        r, elapsed = exchange(c, 3002, speech)
        assert not r["error"] and r["transcript"]["text"]
        results.append(dict(case="fresh-worker-after-kill", wall_ms=elapsed, result=r))
        c.stdin.close()
        c.wait(timeout=10)
    finally:
        if c.poll() is None:
            c.kill()
            c.wait(timeout=10)
        stop.set()
        thread.join(timeout=2)
    observations["working_directory_files"] = sorted(str(x.relative_to(cwd)) for x in cwd.rglob("*") if x.is_file())
    assert not observations["working_directory_files"]
    report = dict(status="pass", platform="Linux x86_64 CPU", scope="pipe worker only; not desktop session/insertion tests",
                  resource_observation_status="unverified: confirm /proc PID mapping before treating observations as RSS/network measurements",
                  model=model.split("/")[-1], silence_requests=500, speech_requests_completed=7,
                  invalid_prompt_recovery=True, kill_and_restart=True, observations=observations, results=results)
    (a.output / "worker-smoke.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "results"}, ensure_ascii=False))


if __name__ == "__main__":
    main()
