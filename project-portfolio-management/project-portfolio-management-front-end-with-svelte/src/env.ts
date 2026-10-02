import { defineEnvVars } from "@sveltejs/kit/env";

// @migration-task Review usage of dynamic environment variables. They fall back to the empty string if not present, which may not be what you want.
export const variables = defineEnvVars({
  PUBLIC_OIDC_SIGNIN_ENABLED: { public: true, schema: (input) => input ?? "" },
  PROJECT_PORTFOLIO_MANAGEMENT_API_URL: { schema: (input) => input ?? "" },
  AUTH_API_URL: { schema: (input) => input ?? "" },
});
