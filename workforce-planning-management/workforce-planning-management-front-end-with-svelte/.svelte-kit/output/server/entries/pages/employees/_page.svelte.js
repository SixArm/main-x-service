import { e as escape_html } from "../../../chunks/escaping.js";
import "clsx";
import "@sveltejs/kit/internal";
import "../../../chunks/exports.js";
import "../../../chunks/utils2.js";
import "@sveltejs/kit/internal/server";
import "../../../chunks/root.js";
import "../../../chunks/state.svelte.js";
import { setEnv } from "@svar-ui/lib-dom";
import "@svar-ui/core-locales";
import { env } from "@svar-ui/lib-svelte";
import "@svar-ui/grid-locales";
import "@svar-ui/lib-state";
import "@svar-ui/grid-store";
import { f as attr, h as attr_class, c as clsx, d as derived, k as stringify } from "../../../chunks/index2.js";
import { t } from "../../../chunks/i18n.svelte.js";
function Button($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      type = "",
      css = "",
      icon = "",
      disabled = false,
      title = "",
      tooltip,
      text = "",
      children,
      onclick
    } = $$props;
    let buttonCss = derived(() => {
      let cssType = type ? type.split(" ").filter((a) => a !== "").map((x) => "wx-" + x).join(" ") : "";
      return css + (css ? " " : "") + cssType;
    });
    $$renderer2.push(`<button${attr("title", title)}${attr_class(`wx-button ${buttonCss()}`, "svelte-z2o62d", { "wx-icon": icon && !children })}${attr("disabled", disabled, true)}${attr("data-tooltip-text", tooltip)}>`);
    if (icon) {
      $$renderer2.push(`<!--[0--><i${attr_class(clsx(icon), "svelte-z2o62d")}></i>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    if (children) {
      $$renderer2.push("<!--[0-->");
      children($$renderer2);
      $$renderer2.push(`<!---->`);
    } else {
      $$renderer2.push(`<!--[-1-->${escape_html(text)}`);
    }
    $$renderer2.push(`<!--]--></button>`);
  });
}
setEnv(env);
const handlers = {};
function registerToolbarItem(type, handler, config = null) {
  handlers[getID(type, config)] = handler;
}
function getID(type, config = null) {
  return `${type}${config?.menu ? ":menu" : ""}`;
}
function Separator($$renderer, $$props) {
  let { menu = false } = $$props;
  $$renderer.push(`<div${attr_class(`wx-separator${menu ? "-menu" : ""}`, "svelte-nejrft")}> </div>`);
}
function Spacer($$renderer) {
  $$renderer.push(`<div class="wx-spacer svelte-3zgmbg"></div>`);
}
function Label($$renderer, $$props) {
  const { text, value, children } = $$props;
  if (children) {
    $$renderer.push(`<!--[0--><div class="wx-label svelte-o8umpq">`);
    children($$renderer);
    $$renderer.push(`<!----></div>`);
  } else {
    $$renderer.push(`<!--[-1--><div class="wx-label svelte-o8umpq">${escape_html(value || text)}</div>`);
  }
  $$renderer.push(`<!--]-->`);
}
function Icon($$renderer, $$props) {
  let {
    icon,
    title,
    text,
    tooltip,
    css,
    type,
    disabled,
    menu,
    onclick
  } = $$props;
  if (menu) {
    $$renderer.push(`<!--[0--><div${attr_class("wx-item svelte-1awua6h", void 0, { "wx-text-icon": text })}><i${attr_class(`${stringify(icon || "wxi-empty")} ${stringify(css || "")}`, "svelte-1awua6h")}></i> ${escape_html(text)}</div>`);
  } else {
    $$renderer.push("<!--[-1-->");
    Button($$renderer, { icon, type, css, disabled, title, tooltip, onclick });
  }
  $$renderer.push(`<!--]-->`);
}
function Item($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { id = "", text = "", css = "", icon = "", onclick } = $$props;
    $$renderer2.push(`<div${attr_class(`wx-label ${stringify(css)}`, "svelte-yzbe9f")}>`);
    if (icon) {
      $$renderer2.push(`<!--[0--><i${attr_class(clsx(icon), "svelte-yzbe9f")}></i>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> ${escape_html(text)}</div>`);
  });
}
registerToolbarItem("button", Button);
registerToolbarItem("button", Icon, { menu: true });
registerToolbarItem("separator", Separator);
registerToolbarItem("spacer", Spacer);
registerToolbarItem("label", Label);
registerToolbarItem("item", Item);
registerToolbarItem("icon", Icon);
setEnv(env);
(/* @__PURE__ */ new Date()).valueOf();
setEnv(env);
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    $$renderer2.push(`<h1>${escape_html(t("nav.employees"))}</h1> `);
    {
      $$renderer2.push(`<!--[1--><p>${escape_html(t("common.loading"))}</p>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
