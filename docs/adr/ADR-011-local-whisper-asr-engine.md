# ADR-011: Local Whisper as the default ASR engine

**Status:** Accepted. Build feasibility verified; engine not yet integrated.

## Context

Audis needs speech recognition for exactly two languages, Indonesian and
English, and it needs to be cheap enough that a user can run it all day without
thinking about cost. Cloud ASR priced per minute makes an always-on transcription
tool expensive to use and expensive to demo.

Restricting the product to `id` and `en` is a real advantage rather than a
limitation. Language can be forced instead of detected, which removes a failure
mode and a round trip, and smaller models are viable because they no longer have
to carry 97 languages.

## Decision

**Local Whisper (whisper.cpp via `whisper-rs`) is the default engine.** It is
free, runs offline, needs no API key, and handles both target languages well.
Models are downloaded on demand from within the application rather than bundled,
so the installer stays small.

Cloud engines remain available behind the same `AsrProvider` interface for users
who want them, but nothing in the product requires one. OpenAI is not the
default and not privileged.

### Build requirements

whisper.cpp is C++, so building Audis from source needs more than Rust. Verified
working on 2026-07-15:

| Requirement       | Detail                                                   |
| ----------------- | -------------------------------------------------------- |
| MSVC              | VS 2022 Build Tools (14.44). 2019's cmake is too old.    |
| cmake             | 4.2. Must support the `Visual Studio 17 2022` generator. |
| Ninja             | Any. Shipped with VS.                                    |
| libclang          | Required by `bindgen`. `LIBCLANG_PATH` must point at it. |
| `CMAKE_GENERATOR` | `Ninja`. Avoids the VS generator instance mismatch.      |
| Path length       | The repo must live in a short path.                      |

None of this reaches end users. The shipped `Audis.exe` has no such
dependencies; this is build-time only, and CI runners already have LLVM.

**Path length is the trap.** MSVC still enforces `MAX_PATH` (260 characters).
CMake's try-compile scratch directories nest deeply, so a repo checked out to a
long path fails with a misleading error:

```
fatal error C1083: Cannot open compiler generated file: '': Invalid argument
```

That message names no path and suggests nothing about length. It cost most of a
day. Keep the checkout near the drive root, for example `C:\Projects\Audis`.

## Consequences

- Transcription is free and offline by default, and works with no account.
- Users choose between local and cloud, and the choice is visible before a
  session starts.
- Building from source needs a C++ toolchain, which raises the barrier for
  contributors. Documented in `setup.ps1`, which checks for each requirement.
- Model files are a runtime download, so first use needs a network connection
  even though later use does not.
- Whisper is not natively streaming. Live captions come from VAD-driven chunked
  decoding, which is how streaming Whisper is done everywhere; latency is
  roughly one utterance rather than one word.

## Alternatives considered

- **OpenAI Realtime:** good, streaming, and priced per minute. Rejected as the
  default because an always-on tool should not meter the user. Still available
  as a choice.
- **Groq / Gemini:** cheap, and Gemini has a free tier. Kept as cloud options,
  but neither can be the default because both need an account and a network.
- **sherpa-onnx:** avoids nothing. It also needs bindgen and a C++ build, and
  its Indonesian coverage is weaker than Whisper's.
- **Bundling a model in the installer:** rejected. Even a small model is tens of
  megabytes and most users need only one of them.
