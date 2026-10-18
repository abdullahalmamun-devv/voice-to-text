# Project status — 18 October 2026

The recovered, checksum-verified baseline is 0.2.0-alpha.1. Its historical evidence is in QA-REPORT.md; tests listed there were run on 11 October, not rerun merely by restoring this archive. The source is being published to https://github.com/abdullahalmamun-devv/voice-to-text.

An earlier alpha.2 work session was not checkpointed before temporary-workspace cleanup. Those edits and unfinished benchmark outputs are unavailable. Planned or previously reported changes must be reconstructed and verified before being called delivered. This repository, rather than temporary storage, will hold subsequent checkpoints.

## Remaining blockers

- Recover priority audio, cancellable WAV, timestamp validation and insertion changes; rerun meaningful checks.
- Complete Bengali/English interface, onboarding and evaluated voice-activity detection.
- Compare intact Small and Turbo weights on a frozen, rights-cleared Bengali corpus. Earlier exploratory wrong-script outputs block accepting Small as an evaluated default. No numerical recognition score from that lost run is claimed here.
- Execute actual Windows microphone, shortcut, focus, insertion/Undo, installer, resource, network, DPI and accessibility QA.
- PRD benchmark gates remain open: 200 utterances/12 speakers for smoke, 1,000/50 for holdout, 200 non-speech clips, 500 nominal insertion trials, 50 guard cases, 100 cancels and an eight-hour run.
- macOS and Linux desktop adapters remain unimplemented; the Linux CLI is not desktop support.

## Planning

Publication and automated build checks are the immediate milestone. Estimate the remaining schedule again after a working alpha.2 checkpoint and initial Windows/model results. The PRD's original Windows planning range is 6–10 active engineering weeks for the full effort, not an elapsed-time promise or a measured amount remaining. Stable completion depends on Windows devices and suitable speech data. Other desktop platforms require separate integration and acceptance work. No 100% ASR or final delivery date is promised.
