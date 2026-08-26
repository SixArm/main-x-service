# Request for comments

What this project wants to learn, and the feedback that helps most.
Send comments to <joel@joelparkerhenderson.com> or open an issue.

## Open questions we want outside views on

- **Matching quality in the wild.** The matcher crates' weights and
  thresholds are literature-derived defaults. Real-world precision /
  recall reports — especially for non-Western names, addresses, and
  identifier schemes — would directly improve them.
- **Event-matcher window scoring.** Should time-bounded events score
  the `[start, end]` window-overlap fraction instead of Gaussian decay
  over endpoint distance? (event-matcher `spec/10-open-questions.md`
  OQ-C.)
- **FHIR surface priorities.** Which resources, search parameters, and
  operations would a real integrator need next? (See
  [agents/share/fhir.md](agents/share/fhir.md) for what exists.)
- **Post-quantum token posture.** The analysis in
  [agents/share/authentication-sessions.md](agents/share/authentication-sessions.md)
  §5.1 concludes "be ready, don't switch yet" — challenges welcome.
- **Comparisons.** Systems missing from
  [COMPARISONS.md](COMPARISONS.md) that this should be evaluated
  against.

## Feedback that is always useful

- A spec claim you found to be untrue in code (the repo treats this as
  a first-class defect).
- A tutorial step that did not work as written.
- Deployment experience reports: what broke, what was unclear, what
  the runbooks missed.
