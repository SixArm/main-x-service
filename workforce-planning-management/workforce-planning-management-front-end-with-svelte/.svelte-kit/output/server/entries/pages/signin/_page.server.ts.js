import { r as requestMagicLink } from "../../../chunks/auth.js";
const load = () => {
  return { title: "Sign in — Workforce Planning Management" };
};
const actions = {
  default: async ({ request, fetch, url }) => {
    const form = await request.formData();
    const email = String(form.get("email") ?? "").trim();
    if (!email) {
      return { sent: false, error: "email-required" };
    }
    const ok = await requestMagicLink(fetch, email, url.origin);
    return ok ? { sent: true, error: null } : { sent: false, error: "failed" };
  }
};
export {
  actions,
  load
};
