# Architecture and design specification

Rust core with a native Win32 desktop adapter and whisper.cpp CPU inference. The product has no WebView, local web server, socket IPC, telemetry, account backend, downloader, updater or remote post-processing. Audio/sample requests travel over anonymous stdin/stdout pipes to one reusable child of the same executable (`--worker`). Model context is retained until cancellation, exit or the configured idle saver. A bounded one-request queue prevents unbounded background jobs.

## Data flow and ownership

1. A user button/shortcut explicitly starts WaveIn capture, mono PCM16 at 16kHz. Four stable boxed buffers feed a bounded RAM sample vector; microphone cap is 120s.
2. Stop closes the microphone before queuing transcription. RMS gating rejects near-silence; this is not trained VAD or a confidence score. The worker holds one multilingual model and creates an inference state for each request.
3. A monotonically changing session ID links requests/results. Cancellation invalidates the ID, closes capture, kills/reaps the worker and clears retained audio/result/target. Late or preview results cannot become a new session's final output.
4. Dictionary context aids decoding; deterministic non-chained exact aliases and conservative formatting process the final text. No LLM rewriting/translation is added.
5. Review is default. Automatic insertion requires an unchanged identifiable native field. Explicit Insert first arms an action; user chooses a field and uses/releases the shortcut. Native field snapshots store HWND/process/thread/caret and a SHA256 content digest, not a disk copy. Clipboard changes only on explicit Copy.
6. Read-back verifies delivered text and caret after a delay. Failed/unconfirmed delivery holds the result without automatic retry. Undo requires the exact expected field contents/caret and deletes only the known UTF-16 range. It is not a general editor undo.
7. Settings/dictionary JSON are local; optional history contains transcripts only, never audio. Export and diagnostics require a user action. Diagnostics exclude audio/transcripts/dictionary/user paths. There is no automatic logging of speech.

Audio remains in app RAM for Retry until next capture, Discard/Cancel or Exit. The operating system may page/back up memory or clipboard; the app does not promise forensic erasure. History files are plaintext and capped; retention expiry is applied when reading/appending history, not by an always-running background cleanup service.

## Surfaces and states

| Surface | Content and behavior |
|---|---|
| Dictation | Mic selection, Start/Stop, Cancel, editable final result, Copy/Insert/Retry/Discard/Undo and WAV/TXT/MD/SRT actions |
| Preferences | Language, digits, shortcut/modifiers/mode, review, optional drafts/commands/history/retention, model saver, theme, startup and manual imports/exports |
| Dictionary & snippets | Visible preferred terms and exact aliases; confirm corrections with Add/update; explicit snippet triggers |
| Local history | Opt-in plaintext transcript list, read-only detail, explicit Copy/export/Delete all |
| Recording surface | Nonactivating topmost indicator; main status shows elapsed time/level/120s warning; no external draft commits |
| Tray | Open, start/stop, Cancel and Exit; right-click ends active recording before the nested menu loop |

Core states: Ready → Listening → Processing → Held/Ready or Error. Duplicate starts and duplicate/late finishes are rejected. Result actions Insert/Retry/Undo and modal file operations are blocked during active capture/processing. A callback guard prevents mutable app reentry during native nested modal loops. Escape is registered while active. Windows session lock/logoff and suspend handlers cancel.

Native font: Nirmala UI body 17px and Segoe UI heading 27px at 96DPI. Window layout scales on DPI changes; system/light/dark colors exist. Controls provide native keyboard/accessibility semantics, but accessibility and theme/DPI rendering have not been tested on Windows. No fabricated confidence percentage is shown.

## Adapter limits

Native Unicode Edit/RichEdit controls ≤32768 UTF-16 units and insertions ≤8000 units are inspected. Selected text, password/read-only/disabled/ANSI/unknown controls are blocked. Browser, modern custom controls, terminals and elevated targets require manual Copy. Focus checks plus SendInput cannot make a universal zero-race guarantee; real guard tests are mandatory. No Enter/Send virtual key is injected; formatting uses Unicode characters.

WAV import is synchronous and capped at 250MB/10min. It averages channels and linearly resamples to 16kHz; a better anti-alias resampler is a remaining quality improvement. SRT uses only decoder timestamps and rejects non-positive/overlapping ranges; it does not invent word timing or time edited text.

The GNU Windows model loader uses a temporary byte buffer to support Bengali/Unicode paths; buffer memory is freed after context construction. This temporarily adds the model file size to peak loading memory and must be measured on the target device. CPU portability is preferred over AVX in this package; performance is not guaranteed.
