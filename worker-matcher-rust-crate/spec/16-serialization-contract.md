## 16. Serialization Contract

All public types in §11 except `MatchingEngine` MUST be `Serialize + Deserialize`. JSON is the reference format; `serde_json` is a hard dependency. Optional fields round-trip `null` ⇄ `None`. Dates serialise as ISO-8601 strings via `jiff`'s `serde` feature. `MatchConfig` carries `#[serde(default)]` on the struct so partial JSON merges over `MatchConfig::default()`. `SimilarityAlgorithm` serialises as the bare variant name. `NicknameTable` serialises as `{ "classes": [["michael", "mike", "mickey"], …] }`; entries are pre-normalised at insertion time so the round-trip is byte-stable.

---

