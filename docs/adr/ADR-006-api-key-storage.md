# ADR-006: API key storage

**Status:** Accepted. Not yet implemented.

## Context

Audis holds provider credentials that can bill the user's account. They must
survive updates, never leak, and never be recoverable from a settings file, a
log, an export, or a crash report.

## Decision

Secrets are stored in an **OS-backed secret store** (Windows Credential
Manager) and accessed **only** from the Rust `audis-security` module. Settings and
SQLite store a **reference**, never a value:

```
credential_ref = "provider/openai/default"
```

Rules:

- The frontend never receives a key. Provider calls are made by Rust.
- Keys are never written to logs, SQLite, exports, or crash reports.
- Keys are masked in the UI and are **never displayed after saving**. They can
  be replaced or deleted, but not read back.
- One provider's key is never sent to another provider.
- In BYOK mode, keys are never sent to the Neura Audis control plane.
- Secret buffers are zeroized (`zeroize`) where practical.
- A "test connection" action validates a key without revealing it.

Two commercial modes are supported by the same abstraction: **BYOK** (the app
uses the user's keys directly) and **Neura Audis managed** (the app receives
short-lived tokens from the control plane). No permanent Neura Audis provider
key is ever embedded in the desktop executable.

## Consequences

- Keys are protected by the OS user account and survive reinstalls.
- "Show my key" is impossible by design. Replacement is the flow instead.
- All provider I/O must live in Rust, which is required anyway.

## Alternatives considered

- **Encrypted file with an app-embedded key:** the key ships in the binary, so
  it is not a secret. Rejected.
- **DPAPI directly:** viable, but Credential Manager gives better semantics and
  user visibility. Credential Manager preferred; DPAPI remains a fallback.
