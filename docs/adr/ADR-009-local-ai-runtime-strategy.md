# ADR-009: Local AI runtime strategy

**Status:** Accepted. Not yet implemented.

## Context

Audis must offer local ASR (privacy, offline, cost) and local speaker/VAD models
without turning the installer into a developer-environment setup task.

## Decision

Run all local inference **natively**, behind an engine interface:

- **Local ASR:** whisper.cpp or a compatible native Whisper runtime.
- **Lightweight local models:** ONNX Runtime.
- **Speaker processing:** sherpa-onnx or equivalent ONNX models.
- **VAD:** a local VAD model or a mature VAD implementation.

Constraints:

- **No AI workload runs in the WebView.**
- **Prefer native Rust or C/C++ FFI over Python** for the shipped product. The
  production installer must not require the user to install Python. Python is
  allowed for research scripts and model evaluation only.
- A **native sidecar is permitted** only when a library cannot be safely
  embedded in Rust, and then it must: be bundled and versioned with Audis, be
  started/stopped by Audis, communicate only over an authenticated local channel
  (loopback or named pipe, random per-launch token), never expose an
  unauthenticated port, emit structured logs, shut down cleanly, and sit behind
  a replaceable interface.
- Models are downloaded to `%LOCALAPPDATA%\NeuraAudis\Audis\models\` and managed
  by an in-app model manager, not bundled into the installer.

## Consequences

- Installer stays small and self-contained; no Python runtime to support.
- Local ASR quality/latency depends on user hardware; the model manager must be
  honest about requirements, and accuracy is never promised as perfect.
- FFI means `unsafe` in the engine crates; it is confined there and never in
  `audis-common` (which forbids `unsafe_code`).

## Alternatives considered

- **Python + PyTorch sidecar:** best model ecosystem, catastrophic distribution
  story (multi-GB, environment fragility). Rejected for the shipped product.
- **Cloud-only:** simplest, but abandons the privacy and offline promises that
  differentiate Audis. Rejected.
