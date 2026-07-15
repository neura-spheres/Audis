# ADR-008: Signed GitHub Release updates

**Status:** Accepted. Not yet implemented.

## Context

Audis must update itself safely on user machines. An update channel is the most
dangerous surface a desktop app has: whoever controls it controls the machine.

## Decision

Use the **official Tauri updater** with **GitHub Releases** as the first
distribution system, and **cryptographic signatures as the only trust anchor**.

- The updater public key is **pinned in the application**.
- Every artifact's signature is **verified before installation**.
- HTTPS endpoints only.
- A GitHub release name/tag is **never** treated as proof of authenticity.
- Unsigned update scripts are never executed. Audis never updates by downloading
  and running an arbitrary PowerShell script, PowerShell orchestrates
  _publishing_, never _installing_.
- Separate `stable` and `beta` channels with distinct metadata.
- Silent downgrade is prevented unless explicitly allowed for recovery.
- Update history is recorded; a post-update health check runs after restart.
- The release workflow **fails closed** when signing credentials are absent for
  a production release.

## Consequences

- Compromising the GitHub account is insufficient to ship a malicious update
  the signing key is also required.
- The signing key becomes the critical secret; it lives only in CI secrets and
  offline backup, never in the repository.
- Publishing requires a draft-then-validate-then-publish flow so a partial
  upload can never become a live release.

## Alternatives considered

- **Custom update server:** more control, more infrastructure and more attack
  surface for no v1 benefit. Deferred; the updater abstraction allows it later.
- **Trust the release tag / checksums only:** checksums prove integrity, not
  authorship. Rejected as the trust anchor (still published for convenience).
