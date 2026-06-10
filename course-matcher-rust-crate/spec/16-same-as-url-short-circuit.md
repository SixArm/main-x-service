## 16. Same-as URL short-circuit

Any pair of `same_as` URLs that fold to the same string short-
circuits. The fold normalises scheme case + host case + path; we do
NOT strip trailing slashes (the URL `/` carrier matters in
canonical schema.org links).

