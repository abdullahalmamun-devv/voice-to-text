# Privacy and network evidence

The implemented product code uses local model files, microphone capture, pipe IPC, local JSON files and native text/clipboard APIs. It contains no network transport client, updater, telemetry, URL opener, remote model fallback or local listener. Shipped PE imports do not include Winsock/WinHTTP/WinINet. This static evidence supports the offline design; it is **not a substitute for Windows app-owned process-tree traffic capture**.

| Data | Default location/lifetime | Writes |
|---|---|---|
| Microphone samples | App/worker RAM, one current recording | No app-owned audio-file persistence; held for Retry until next recording/Discard/Cancel/Exit |
| Raw/final transcript | Current app result in RAM | Only explicit export or enabled local history |
| Dictionary/snippets/preferences | User profile local JSON + previous settings backup | Explicit changes/imports |
| History | Disabled by default; plaintext local JSON when enabled | Max 500 entries, optional 7/30/manual retention; no audio |
| Target text used for insertion checks | Temporary reads/owned Undo in RAM | No target-text logging or file writes |
| Clipboard | Windows clipboard | Explicit Copy only; user/OS clipboard tools may persist it |
| Diagnostics | Explicit selected local JSON path | Version/platform/timing/runtime info; no speech/text/dictionary/user paths |

Default-off history was unit-tested to leave its directory empty after attempted transcript append. Worker integration left its working directory empty. Neither covers every Windows filesystem path/OS behavior. Development benchmark exports intentionally contain reference/hypothesis texts; use only consented test data and store those reports appropriately.

Pending Windows procedure: capture the app and all product child processes from clean launch through model load, mic record, preview, final transcription, cancellation, Copy/Insert/Undo, opt-in history, diagnostics/export, idle unload and exit; repeat with network enabled/disabled. Record tool/version/OS/process IDs/endpoints and file writes. OS-originated traffic must be separated from app-owned traffic. Pass requires no product requests/listeners. No packet capture has been completed here; descriptor inspection was restricted. Do not label this report “zero-network runtime certified.”

No app-level history encryption or secure-deletion guarantee is implemented. Per-user files inherit Windows profile permissions; Unix core tests use 0600 files. OS paging, crash dumps, clipboard history, backups and files saved by the target app remain outside application control.
