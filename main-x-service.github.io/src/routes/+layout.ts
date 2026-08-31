// Fully static, unauthenticated docs site — prerender every route at build
// time. No SSR data fetching, no client-side auth: this is a read-only
// public front door, not one of the family's operator front-ends.
export const prerender = true;
export const trailingSlash = 'always';
