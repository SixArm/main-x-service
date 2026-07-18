# Regulatory posture

> ⚠️ **Demo software.** Not a production CRM; no real personal
> data; no real email is sent (delivery is simulated).

## Observed by design

- **Consent-first marketing** — no send without recorded, current
  consent; unsubscribe is immediate, permanent until re-grant, and
  bypasses nothing (the send path enforces it, not the UI). The
  append-only consent history is the compliance evidence.
- **Data minimisation** — contacts/accounts are URN wrappers;
  demographics stay in the identity registries; CRM holds
  relationship state.
- **Access control** — ABAC personas + amount/channel masking;
  the `CRM_REQUIRE_AUTH` activation gate must be on before real
  exposure.
- **Auditability** — mutations + consent history + sensitive reads.
- **Synthetic data only** in seeds and tests.

## Production would additionally require

- UK/EU GDPR + PECR (e-privacy) compliance for the real send path:
  lawful basis per channel, soft opt-in rules, sender
  identification, working unsubscribe in every message; ESP
  contract terms (processor agreements); retention schedules for
  leads/activities/tickets; subject-access and erasure coordinated
  with the identity services; and — if scoring ever influences
  significant decisions — automated-decision-making review.
  Tracked as production gates in [tasks.md](tasks.md).
