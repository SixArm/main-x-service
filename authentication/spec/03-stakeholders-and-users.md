## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| Peer services (relying parties) | Every other Main X Index service (person, worker, place, thing, event, course, organization, care-pathway) accepts this entity's tokens. They embed the verifier crate and depend on a stable JWT claim set, a stable JWKS shape, and predictable key rotation. |
| Operators / end users | Humans who sign into any Main X front-end. They need fast, reliable, localised sign-in at governmental population scale — and no passwords to manage. |
| Sibling front-ends | The auth front-end performs the flow; sibling front-ends consume the stored bearer token against their own services. |
| Security team | Key custody and rotation, token lifetime policy, anti-enumeration behaviour, abuse / rate-limit posture, incident response. |
| Auditors / regulators | GDPR / UK DPA / ISO 27001 evidence: who authenticated when, issuance and revocation trails, personal-data minimisation (email addresses). |
| Operations / DBA | PostgreSQL schema + migration discipline; the auth service is the availability-critical hub — its uptime and key-serving posture gate the whole federation's sign-in. |
| Developers / agents | The service is the family's **reference loco.rs application**; the verifier is the reference for adding JWT enforcement to peer services during the loco conversion. |
