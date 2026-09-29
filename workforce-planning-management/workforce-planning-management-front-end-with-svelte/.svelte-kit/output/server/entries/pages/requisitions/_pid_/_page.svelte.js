import { e as escape_html } from "../../../../chunks/escaping.js";
import "clsx";
import "../../../../chunks/state.svelte.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils2.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import { t } from "../../../../chunks/i18n.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    {
      $$renderer2.push(`<!--[1--><p>${escape_html(t("common.loading"))}</p>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
