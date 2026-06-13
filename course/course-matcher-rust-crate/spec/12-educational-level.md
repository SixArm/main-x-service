## 12. Educational level

| Pair | Score |
|---|---|
| Same variant | 1.0 |
| Adjacent on the skill ladder (`Beginner < Intermediate < Advanced < Expert`) | 0.5 |
| Adjacent on the school ladder (`Primary < Secondary < Higher`) | 0.5 |
| Adjacent on the degree ladder (`Undergraduate < Graduate < Postgraduate`) | 0.5 |
| Across ladders | 0.0 |
| Either side `None` | component skipped |

`EducationalLevel::Custom(s)` is compared by equality on the inner
string.

