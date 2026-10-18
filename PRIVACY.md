# Privacy

The product is designed for offline local inference with no account, transcription API, updater, downloader or telemetry. A complete process-tree network capture on Windows is still required before stable release.

Microphone audio stays in a bounded memory buffer for the current session and explicit Retry. Cancel, discard, a new recording or Exit releases that buffer. Audio is not automatically recorded to disk. The operating system can page memory or create crash dumps; this application cannot promise that the entire computer leaves no trace.

Settings and dictionary entries are local. Transcript history is off by default; enabling it stores plaintext locally with the selected retention and a 500-entry cap. Manual exports and explicit Copy are user actions. Clipboard history, backups and receiving applications have their own behavior.

Native insertion checks only the selected supported text field to validate identity, caret and content. Unknown controls use explicit Copy. It does not read unrelated applications in the background. See docs/PRIVACY-AND-NETWORK.md for the exact boundary and pending verification.
