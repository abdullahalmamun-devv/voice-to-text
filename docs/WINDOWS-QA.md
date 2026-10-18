# Windows release test register — execution pending

Run on a real Windows 11 x64 machine. Record OS build, CPU/RAM, microphone, display/DPI, app/model hash, exact settings, actual result and evidence path for each case. Nothing below is marked passed by cross-compilation.

| Group | Required actions | Pass evidence |
|---|---|---|
| Fresh install | Disconnect network; verify/extract full archive; run portable and per-user install; include Bengali username/path and spaces | Model loads; no runtime download/admin/account required; first microphone test completes |
| Microphones | Default/USB/Bluetooth/other device, no permission, unplug/plug, exclusive device contention | Clear failure/recovery; selected device used; capture never continues unnoticed |
| Shortcut | Default and alternate modifiers/key; conflict; key repeat; quick press; hold-release; toggle; button and tray | One session; release stops capture; modifiers do not modify inserted text |
| State/cancellation | Cancel during capture/model load/decode/preview; repeated start; immediately record again; Exit; 120s cap | Mic stops; stale IDs ignored; no old result inserted; worker reaped/restarted |
| Lock/sleep | Lock/suspend/logoff/restore with active capture/decode; open tray menu during recording | Capture cancelled; no ghost sessions on resume; meter/overlay truthful |
| Nominal insertion | At least 500 Bengali/mixed/number/multiline trials across identified native control variants | ≥99% measured nominal success for published supported matrix; delivery read-back correct |
| Guard suite | At least 50 tests: change window/control/caret/text/selection, password/read-only/disabled, hung target, elevated target, target closes | No known destructive wrong-target insertion; result held; no selected text overwrite |
| Clipboard | Unicode and multiple formats, busy clipboard, clipboard manager, Copy failure | Automatic insertion leaves clipboard unchanged; explicit Copy works; failure clear |
| Delivery/Undo | Partial/blocked SendInput, user types during delay, Undo original/changed/closed field, modifier release | No repeated automatic delivery; Undo only exact owned range; no unrelated deletion |
| Browser/editor | Chrome/Edge/Firefox, new Notepad, Word, terminals and custom editors | This alpha uses manual Copy fallback for unrecognized controls; record actual compatibility, do not promote to auto-insertion tier |
| Review/draft | Default review; no-review supported native field; fast/noisy speech; stop during draft; result edit | Draft external text untouched; visible final review; bounded worker backlog; no premature Insert/Undo while recording |
| Dictionary/commands | Alias boundaries, exact overlap, digits in protected names, correction proposal/confirm, snippets, literal command mention | User-controlled substitutions; no hidden learning; unrelated app text never edited |
| History/privacy | Off/on, 7/30/manual, expiry, >500 entries, corrupt/oversize files, delete all, history disabled with old entries | No default speech persistence; explicit history only; clear corrupt-data errors; deletion/retention behavior correct |
| WAV/export | PCM8/16/24/32/float mono/stereo, malformed/oversize/empty WAV, TXT/MD/raw SRT after edit | Limits/errors correct; local-only export; timing truthful; no edited-text SRT claim |
| Settings/updates | Import invalid/new schema, write denied/full disk, backup recovery, move package, update/rollback, startup opt-in/remove | Original data preserved; failure not reported as saved; recovery works without network |
| UX/accessibility | Bengali conjuncts, selection/combining marks, Tab/ShiftTab/Space, screen reader; 100/125/150/200% DPI; light/dark/system; small display | No clipped required controls, readable contrast/focus, usable nontechnical flows |
| Offline/resource | Capture process-tree traffic/files; cold/warm timings, idle CPU, app+worker RAM; 500 actual sessions, 100 cancels and extended use | PRD frozen budgets/gates met; no traffic/listeners, mic leaks, process leaks or unbounded memory |
| Bengali quality | Clean/noisy/accent/mixed/names/numbers/negation/repetition/quiet/silence with held-out speakers | Published raw/normalized scores, critical entities, correction effort and sample counts; no fabricated accuracy |

These procedures supplement the original PRD test register. Browser UI Automation, onboarding/UI-language completion, trained VAD, higher quality resampling and platform adapters are implementation work, not tests that can be marked passed without adding them.
