# Audis Security

Audis captures sensitive audio and holds provider credentials. Security is a
product requirement, not a feature. This document records what is enforced today
and what each later milestone adds.

## Threat model (summary)

| Asset                   | Threat                                  | Control                                             |
| ----------------------- | --------------------------------------- | --------------------------------------------------- |
| Provider API keys       | Theft via logs, exports, frontend, disk | OS keystore only; never in frontend/logs/exports    |
| Transcript content      | Prompt injection into the AI layer      | Treated as untrusted quoted material (Milestone 6)  |
| The renderer (WebView2) | Remote code / data exfiltration         | Strict CSP, no remote scripts, minimal capabilities |
| Update artifacts        | Malicious update                        | Signature verification + pinned key (Milestone 8)   |
| Local sidecars          | Unauthenticated local access            | Loopback + per-launch token (when introduced)       |

## Enforced in Milestone 0

- **Tauri capabilities** are minimal: `core:default` on the single `main`
  window. No shell execution, no unrestricted HTTP, no broad filesystem scope.
  Permissions are added per feature, never pre-emptively.
- **Content Security Policy** is strict. `default-src` and `script-src` are
  `'self'` (no remote or inline scripts), `object-src` is `'none'`, and
  `frame-ancestors` is `'none'`. The UI is bundled locally and never loaded
  from a remote origin.
- **Prototype freeze** is on; the Tauri asset protocol is disabled until a
  feature requires it.
- **DevTools** are compiled out of release builds (gated behind the `devtools`
  Cargo feature).
- **The frontend is never trusted.** Every command validates its arguments in
  Rust; every result is schema-validated in TypeScript.
- **Errors are sanitised.** `UserFacingError` carries no stack trace and no
  developer path; technical detail is opt-in behind a disclosure and still
  redacted of secrets.
- **Logging is secret-safe by default.** `INFO` level, rolling files under
  `%LOCALAPPDATA%`, third-party crates quietened. API keys, auth headers,
  provider payloads, raw audio and voice profiles are never logged (the
  provider crates that will hold secrets are written to never pass them to
  `tracing`).
- **`unsafe_code` is forbidden** in `audis-common`; the desktop crate warns on
  `unwrap`/`expect` in non-test code.

## Added by later milestones

- **M7, Secrets.** Windows Credential Manager via `audis-security`. Settings
  store only a `credential_ref` such as `provider/openai/default`, never a key.
  Secret buffers zeroized with `zeroize` where practical.
- **M6, AI boundary.** System instructions kept separate from transcript
  content; transcript marked as untrusted quoted meeting material; no tool
  action without validation and user permission; keys never sent to the model.
- **M8, Updates.** Verify updater signatures before install, HTTPS only, pin
  the updater public key in the app, reject unsigned artifacts, no silent
  downgrade, separate stable/beta channels, recorded update history.
- **M9, Licensing & sidecars.** Locally verified signed entitlements; any
  sidecar binds loopback-only with a random per-launch token, validates
  schemas, and dies with its parent.

## Reporting

Security issues should be reported privately to Neura Audis rather than via
public issues. (Contact channel to be finalised with production infrastructure.)
