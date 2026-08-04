## 15. Roadmap

- **v0.2**: SSR-safe load functions (T-13); Lily Dialog/Combobox integration (T-14); identifier/address edit UI (T-15).
- **v0.3**: ✅ *Done 2026-06-18 (T-22a).* Auth integration via the central `authentication-service` (not gated on Worker Service itself): BFF + httpOnly-cookie + PASETO model per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md) (no client-held bearer / `localStorage`). **Remaining**: CSRF (T-22b).
- **v0.4**: ✅ *Done.* Sibling scaffolds for the Place / Thing / Event front-ends shipped (copy-adapted from this and the person scaffold; accept drift per project decision 2026-06-02).

