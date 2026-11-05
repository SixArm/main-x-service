## 4. Glossary

| Term | Meaning |
|---|---|
| **Course** | The abstract template (course code, name, topics taught). |
| **CourseInstance** | A specific offering of a Course at a particular time / place / mode. |
| **Provider** | The organisation that issues / owns the course. |
| **Deterministic identifier** | An identifier scheme whose values are unique by construction across providers — DOI, Wikidata, LOM (IEEE Learning Object Metadata id), URI, UUID, OER id (`IdentifierType::is_deterministic`). **Not** LMS course-id or a bare course code — both are provider-scoped, not globally unique. |
| **Course code** | A provider-scoped identifier (`CS101`). NOT globally unique. |
| **Envelope** | `{ success, data, error }` wrapper applied to every REST response. |

