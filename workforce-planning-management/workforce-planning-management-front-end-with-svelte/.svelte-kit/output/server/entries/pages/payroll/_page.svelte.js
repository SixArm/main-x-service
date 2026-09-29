import { e as escape_html } from "../../../chunks/escaping.js";
import "clsx";
import { t } from "../../../chunks/i18n.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    $$renderer2.push(`<h1>${escape_html(t("nav.payroll"))}</h1> `);
    {
      $$renderer2.push(`<!--[1--><p>${escape_html(t("common.loading"))}</p>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
