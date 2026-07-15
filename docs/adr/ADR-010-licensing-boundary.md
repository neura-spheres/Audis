# ADR-010: Licensing boundary

**Status:** Accepted. Not yet implemented.

## Context

Audis is commercial and will have Free/Pro/Team/Enterprise plans. Desktop apps
that hardcode a payment provider age badly, and licence checks that punish
users, locking their files when a subscription lapses or a network blips, are
unacceptable for a tool holding hours of their recordings.

## Decision

**The desktop app understands entitlements, not payments.**

- `audis-licensing` verifies a **signed entitlement token** locally and exposes
  an `Entitlements` struct (feature flags, `max_monthly_minutes`,
  `export_formats`, `update_channel`).
- A separately deployable **control plane** (Axum + PostgreSQL + SQLx, signed
  entitlements, OpenAPI) handles identity, device registration, activation,
  entitlement retrieval, short-lived managed-provider token exchange, release
  channel metadata, revocation and audit.
- **No payment provider is hardcoded** into core licensing. Billing integrates
  through a **webhook interface**, so Stripe or anything else is swappable.
- **No permanent Neura Audis provider key ships in the desktop executable.**
  Managed mode exchanges short-lived tokens.
- A **signed development licence** enables local development without the
  control plane.

**Failure is never destructive.** On expiry or revocation: no data is deleted or
locked, existing files stay accessible, and the user can still export and delete
their data. There is an **offline grace period**, and licence state is reported
clearly rather than silently degrading.

## Consequences

- Payment provider can change without touching the client.
- Entitlements are verified offline via signature, so a network outage never
  locks a paying user out mid-meeting.
- Signed tokens mean key management and a revocation story (short expiry +
  `license_cache` + revocation on refresh).
- The control plane is optional for development and for BYOK users.

## Alternatives considered

- **Stripe SDK in the desktop app:** fastest to ship, couples the client to one
  vendor and drags payment concerns into an audio app. Rejected.
- **Online-only licence checks:** simple to reason about, but a network blip
  would break a live meeting. Rejected.
