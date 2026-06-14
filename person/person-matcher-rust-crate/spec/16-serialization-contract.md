## 16. Serialization Contract

All public types in §11 except `MatchingEngine` MUST be `Serialize + Deserialize`. JSON is the reference format (`serde_json` hard dep). Optional fields round-trip `null` ⇄ `None`; dates serialise as ISO-8601 via `chrono`'s `serde` feature. `MatchConfig` carries `#[serde(default)]` so partial JSON deserialises with remaining fields from `MatchConfig::default()`. `SimilarityAlgorithm` serialises as the bare variant name (`"JaroWinkler"` / `"Levenshtein"` / `"Exact"` / `"Combined"`). `NicknameTable` serialises as `{ "classes": [["michael", "mike", "mickey"], …] }` (entries pre-normalised at insertion → byte-stable round-trip).

---

