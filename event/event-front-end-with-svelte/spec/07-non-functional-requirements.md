## 7. Non-Functional Requirements

- **Bundle size**: keep the JS payload of the list page under 250 kB gzipped.
- **Time to first interaction**: under 1 s on a warm dev server.
- **Accessibility**: WAI-ARIA conformance for forms, focus management on navigation.
- **TypeScript**: strict + `noUncheckedIndexedAccess`.
- **No SSR data fetch in MVP**: pages mount and `fetch()` from the browser; SSR fetch is a v0.2 follow-up (§13 T-13).
- **Errors**: every `ApiError` rendered with its `code` and `message`; 422 `details` rendered field-by-field where possible.

