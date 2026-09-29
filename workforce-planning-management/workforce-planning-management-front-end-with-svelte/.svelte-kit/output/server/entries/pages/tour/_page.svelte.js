import "clsx";
import { j as head, f as attr, g as ensure_array_like, d as derived } from "../../../chunks/index2.js";
import { t } from "../../../chunks/i18n.svelte.js";
import { e as escape_html } from "../../../chunks/escaping.js";
function Tour($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { sections } = $$props;
    const STEPS = [1, 2, 3, 4];
    const key = (k) => k;
    const all = derived(() => [
      { id: "start", prefix: "tour.start", href: null },
      ...sections.map((s, i) => ({ id: `s${i + 1}`, prefix: `tour.s${i + 1}`, href: s.href }))
    ]);
    head("1irbmle", $$renderer2, ($$renderer3) => {
      $$renderer3.title(($$renderer4) => {
        $$renderer4.push(`<title>${escape_html(t("nav.tour"))} · ${escape_html(t("brand.name"))}</title>`);
      });
    });
    $$renderer2.push(`<div class="tour svelte-1irbmle" id="top"><header class="tour-head svelte-1irbmle"><p class="eyebrow svelte-1irbmle">${escape_html(t("brand.name"))}</p> <h1 class="svelte-1irbmle">${escape_html(t("tour.head"))}</h1> <p class="lead svelte-1irbmle">${escape_html(t("tour.intro"))}</p></header> <div class="tour-body svelte-1irbmle"><nav class="toc svelte-1irbmle"${attr("aria-label", t("tour.toc"))}><h2 class="svelte-1irbmle">${escape_html(t("tour.toc"))}</h2> <ol class="svelte-1irbmle"><!--[-->`);
    const each_array = ensure_array_like(all());
    for (let i = 0, $$length = each_array.length; i < $$length; i++) {
      let s = each_array[i];
      $$renderer2.push(`<li><a${attr("href", `#${s.id}`)} class="svelte-1irbmle"><span class="num svelte-1irbmle" aria-hidden="true">${escape_html(i + 1)}</span> ${escape_html(t(key(`${s.prefix}.title`)))}</a></li>`);
    }
    $$renderer2.push(`<!--]--></ol></nav> <div class="sections svelte-1irbmle"><!--[-->`);
    const each_array_1 = ensure_array_like(all());
    for (let i = 0, $$length = each_array_1.length; i < $$length; i++) {
      let s = each_array_1[i];
      $$renderer2.push(`<section${attr("id", s.id)}${attr("aria-labelledby", `${s.id}-title`)} class="svelte-1irbmle"><h2${attr("id", `${s.id}-title`)} class="svelte-1irbmle"><span class="num svelte-1irbmle" aria-hidden="true">${escape_html(i + 1)}</span> ${escape_html(t(key(`${s.prefix}.title`)))}</h2> <p class="summary svelte-1irbmle">${escape_html(t(key(`${s.prefix}.summary`)))}</p> <ol class="steps svelte-1irbmle"><!--[-->`);
      const each_array_2 = ensure_array_like(STEPS);
      for (let $$index_1 = 0, $$length2 = each_array_2.length; $$index_1 < $$length2; $$index_1++) {
        let n = each_array_2[$$index_1];
        $$renderer2.push(`<li class="svelte-1irbmle">${escape_html(t(key(`${s.prefix}.step.${n}`)))}</li>`);
      }
      $$renderer2.push(`<!--]--></ol> <p class="section-links svelte-1irbmle">`);
      if (s.href) {
        $$renderer2.push(`<!--[0--><a class="open svelte-1irbmle"${attr("href", s.href)}>${escape_html(t("tour.open"))}</a>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <a class="top svelte-1irbmle" href="#top">${escape_html(t("tour.top"))}</a></p></section>`);
    }
    $$renderer2.push(`<!--]--> <section class="closing svelte-1irbmle" aria-labelledby="tour-cta-title"><h2 id="tour-cta-title" class="svelte-1irbmle">${escape_html(t("splash.cta.title"))}</h2> <p class="svelte-1irbmle">${escape_html(t("splash.cta.body"))}</p> <a class="cta svelte-1irbmle" href="/signin">${escape_html(t("auth.signin"))}</a></section></div></div></div>`);
  });
}
function _page($$renderer) {
  const sections = [
    { href: "/employees" },
    { href: "/requisitions" },
    { href: "/workforce" },
    { href: "/payroll" },
    { href: "/learning" },
    { href: "/wellbeing" }
  ];
  Tour($$renderer, { sections });
}
export {
  _page as default
};
