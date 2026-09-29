import { j as head } from "../../../chunks/index2.js";
import { e as escape_html } from "../../../chunks/escaping.js";
import "@sveltejs/kit/internal";
import "../../../chunks/exports.js";
import "../../../chunks/utils2.js";
import "@sveltejs/kit/internal/server";
import "../../../chunks/root.js";
import { p as public_env } from "../../../chunks/shared-server.js";
import "../../../chunks/state.svelte.js";
import { t } from "../../../chunks/i18n.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { form } = $$props;
    const ssoEnabled = public_env.PUBLIC_OIDC_SIGNIN_ENABLED === "true";
    head("iq265b", $$renderer2, ($$renderer3) => {
      $$renderer3.title(($$renderer4) => {
        $$renderer4.push(`<title>Sign in — Workforce Planning Management</title>`);
      });
    });
    $$renderer2.push(`<h1>Sign in</h1> `);
    if (form?.sent) {
      $$renderer2.push(`<!--[0--><div class="panel"><p>Check your email for a sign-in link.</p></div>`);
    } else {
      $$renderer2.push(`<!--[-1--><div class="panel"><form class="row" method="POST"><label>Email <input type="email" name="email" required="" autocomplete="email"/></label> <button class="primary" type="submit">Send magic link</button></form> `);
      if (form?.error) {
        $$renderer2.push(`<!--[0--><p class="error" role="alert">Could not send the sign-in link. Please try again.</p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> `);
      if (ssoEnabled) {
        $$renderer2.push(`<!--[0--><p><a class="sso svelte-iq265b" href="/signin/sso">${escape_html(t("signin.sso"))}</a></p>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></div>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
