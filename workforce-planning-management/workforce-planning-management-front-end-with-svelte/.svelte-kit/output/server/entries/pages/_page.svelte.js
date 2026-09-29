import "clsx";
import { e as escape_html } from "../../chunks/escaping.js";
import { t } from "../../chunks/i18n.svelte.js";
import { j as head, g as ensure_array_like, f as attr } from "../../chunks/index2.js";
function Dashboard($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    $$renderer2.push(`<h1>${escape_html(t("dash.title"))}</h1> `);
    {
      $$renderer2.push(`<!--[1--><p>${escape_html(t("common.loading"))}</p>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
function Splash($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const TILES = [1, 2, 3, 4, 5, 6];
    const AREAS = [
      { id: "benefits", title: "splash.benefits.title" },
      { id: "features", title: "splash.features.title" },
      { id: "trust", title: "splash.trust.title" }
    ];
    head("v8bbdp", $$renderer2, ($$renderer3) => {
      $$renderer3.title(($$renderer4) => {
        $$renderer4.push(`<title>${escape_html(t("brand.name"))}</title>`);
      });
    });
    $$renderer2.push(`<div class="splash svelte-v8bbdp"><section class="hero svelte-v8bbdp" aria-labelledby="hero-title"><p class="eyebrow svelte-v8bbdp">${escape_html(t("brand.name"))}</p> <h1 id="hero-title" class="svelte-v8bbdp">${escape_html(t("splash.hero.title"))}</h1> <p class="lead svelte-v8bbdp">${escape_html(t("splash.hero.subtitle"))}</p> <div class="actions svelte-v8bbdp"><a class="cta primary svelte-v8bbdp" href="/signin">${escape_html(t("auth.signin"))}</a> <a class="cta secondary svelte-v8bbdp" href="/tour">${escape_html(t("splash.hero.tour"))}</a></div></section> <!--[-->`);
    const each_array = ensure_array_like(AREAS);
    for (let $$index_1 = 0, $$length = each_array.length; $$index_1 < $$length; $$index_1++) {
      let area = each_array[$$index_1];
      $$renderer2.push(`<section class="area svelte-v8bbdp"${attr("id", area.id)}${attr("aria-labelledby", `${area.id}-title`)}><h2${attr("id", `${area.id}-title`)} class="svelte-v8bbdp">${escape_html(t(area.title))}</h2> <ul class="tiles svelte-v8bbdp"><!--[-->`);
      const each_array_1 = ensure_array_like(TILES);
      for (let $$index = 0, $$length2 = each_array_1.length; $$index < $$length2; $$index++) {
        let n = each_array_1[$$index];
        $$renderer2.push(`<li class="tile svelte-v8bbdp"><span class="badge svelte-v8bbdp" aria-hidden="true">${escape_html(n)}</span> <h3 class="svelte-v8bbdp">${escape_html(t(`splash.${area.id}.${n}.title`))}</h3> <p class="svelte-v8bbdp">${escape_html(t(`splash.${area.id}.${n}.body`))}</p></li>`);
      }
      $$renderer2.push(`<!--]--></ul></section>`);
    }
    $$renderer2.push(`<!--]--> <section class="closing svelte-v8bbdp" aria-labelledby="closing-title"><h2 id="closing-title" class="svelte-v8bbdp">${escape_html(t("splash.cta.title"))}</h2> <p class="svelte-v8bbdp">${escape_html(t("splash.cta.body"))}</p> <a class="cta primary svelte-v8bbdp" href="/signin">${escape_html(t("auth.signin"))}</a></section></div>`);
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { data } = $$props;
    if (data.signedIn) {
      $$renderer2.push("<!--[0-->");
      Dashboard($$renderer2);
    } else {
      $$renderer2.push("<!--[-1-->");
      Splash($$renderer2);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
