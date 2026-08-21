## 21. Roadmap and Future Work

**§21.1 Near-term (0.2.x)** — all delivered (T-1 / T-2 / T-3 / T-5 / T-6); see `agents/delivered-tasks.md`.

**§21.2 Medium-term (0.3.x)** — open: T-9.1 (locale-aware phonetic encoder enum, opt-in `MatchConfig::phonetic_encoder` behind `phonetic-rphonetic` feature flag; follow-up to T-9 spike). Delivered: T-10 / T-22 / T-11 / T-24 / T-25.

**§21.3 Longer-term (0.4.x – 1.0)** — optional `match_many_to_many` / blocking-key helpers atop the batch API; optional Fellegi-Sunter training; async batch evaluation; further national-identifier schemes beyond 42 (HK / SG / KR / TR / RU / AR / CA-provincial), incremental per consumer demand; 1.0 API freeze. Declined (`agents/roadmap-research.md`): external postal-address standardisation (T-14); full ~250-territory ITU-T phone expansion + `phonenumber` dep (T-19); per-country mobile/landline phone validation.

### 21.4 Research Spike Outcomes

Full write-ups for the four closed spikes are in [`agents/roadmap-research.md`](../agents/roadmap-research.md). Summary: **T-17** recommended the 7-jurisdiction batch (BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID); shipped as T-17.1, total 42. **T-9** keep Soundex default; add opt-in `phonetic_encoder` (`Soundex` / `DoubleMetaphone` / `DaitchMokotoff`) behind `phonetic-rphonetic`; tracked as T-9.1. **T-19** tactical expansion `COUNTRY_PHONE_TABLE` 26 → 39; declined full ~250 ITU-T expansion, `phonenumber` dep, mobile/landline prefix validation. **T-14** declined external postal-address standardisation at this layer.

---

