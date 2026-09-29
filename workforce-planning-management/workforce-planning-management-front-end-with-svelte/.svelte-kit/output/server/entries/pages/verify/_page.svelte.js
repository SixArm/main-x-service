import { j as head, d as derived } from "../../../chunks/index2.js";
import { e as escape_html } from "../../../chunks/escaping.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { data } = $$props;
    const message = derived(() => data.error === "missingToken" ? "This sign-in link is missing its token." : data.error === "serviceUnavailable" ? "We could not reach the sign-in service. Please try again in a moment." : "This sign-in link is invalid or has expired.");
    head("1230iaq", $$renderer2, ($$renderer3) => {
      $$renderer3.title(($$renderer4) => {
        $$renderer4.push(`<title>Sign-in link — Workforce Planning Management</title>`);
      });
    });
    $$renderer2.push(`<h1>Sign-in link</h1> <div class="panel"><p class="error" role="alert">${escape_html(message())}</p> <p><a href="/signin">Request a new link</a></p></div>`);
  });
}
export {
  _page as default
};
