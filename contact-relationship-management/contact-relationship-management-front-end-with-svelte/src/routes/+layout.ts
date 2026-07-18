// SPA mode (family front-end convention, drift accepted): no SSR, no
// prerender — every route loads client-side through the same-origin
// BFF proxy. This also lets the Playwright suite stub the API with
// `page.route` and run without the Rust service.
export const ssr = false;
export const prerender = false;
