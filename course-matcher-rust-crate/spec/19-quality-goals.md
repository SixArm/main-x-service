## 19. Quality goals

- Zero `unsafe`.
- Zero `unwrap` / `expect` / `panic!` in library code.
- Crate compile time on a warm cache < 5 s.
- `cargo bench` plan covers name match throughput, full
  `match_courses` throughput, and `rank` against N=100.

