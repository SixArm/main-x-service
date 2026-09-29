import { j as head } from "../../../chunks/index2.js";
import { t } from "../../../chunks/i18n.svelte.js";
import { e as escape_html } from "../../../chunks/escaping.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    head("1axyntk", $$renderer2, ($$renderer3) => {
      $$renderer3.title(($$renderer4) => {
        $$renderer4.push(`<title>${escape_html(t("nav.mentorship"))} — WPM</title>`);
      });
    });
    $$renderer2.push(`<h1>${escape_html(t("nav.mentorship"))}</h1> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
