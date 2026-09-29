import { e as escape_html } from "../../../chunks/escaping.js";
import "clsx";
import "@svar-ui/lib-state";
import { uid, setEnv } from "@svar-ui/lib-dom";
import "@svar-ui/core-locales";
import { env } from "@svar-ui/lib-svelte";
import { i as getContext, h as attr_class, k as stringify, f as attr, b as bind_props, c as clsx, d as derived, l as setContext, m as attr_style } from "../../../chunks/index2.js";
import { t } from "../../../chunks/i18n.svelte.js";
function getInputId(id) {
  const contextId = getContext("wx-input-id");
  return id || contextId || uid();
}
function TextArea($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      value = "",
      id,
      placeholder = "",
      title = "",
      tooltip,
      disabled = false,
      error = false,
      readonly = false,
      css = "",
      onchange
    } = $$props;
    const inputId = getInputId(id);
    $$renderer2.push(`<textarea${attr_class(`wx-textarea ${stringify(css)}`, "svelte-1d4i2j2", { "wx-error": error })}${attr("id", inputId)}${attr("disabled", disabled, true)}${attr("placeholder", placeholder)}${attr("readonly", readonly, true)}${attr("title", title)}${attr("data-tooltip-text", tooltip)}>`);
    const $$body = escape_html(value);
    if ($$body) {
      $$renderer2.push(`${$$body}`);
    }
    $$renderer2.push(`</textarea>`);
    bind_props($$props, { value });
  });
}
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
    $$renderer2.push(`<button${attr("title", title)}${attr_class(`wx-button ${buttonCss()}`, "svelte-z21jlc", { "wx-icon": icon && !children })}${attr("disabled", disabled, true)}${attr("data-tooltip-text", tooltip)}>`);
    if (icon) {
      $$renderer2.push(`<!--[0--><i${attr_class(clsx(icon), "svelte-z21jlc")}></i>`);
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
function Checkbox($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      id,
      label = "",
      inputValue = "",
      value = false,
      disabled = false,
      css = "",
      onchange
    } = $$props;
    const inputId = getInputId(id);
    $$renderer2.push(`<div${attr_class(`wx-checkbox ${stringify(css)}`, "svelte-1dakodr")}><input type="checkbox"${attr("id", inputId)}${attr("disabled", disabled, true)}${attr("checked", value, true)}${attr("value", inputValue)} class="svelte-1dakodr"/> <label${attr("for", inputId)} class="svelte-1dakodr"><span class="svelte-1dakodr"></span> `);
    if (label) {
      $$renderer2.push(`<!--[0--><span class="svelte-1dakodr">${escape_html(label)}</span>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></label></div>`);
    bind_props($$props, { value });
  });
}
function Text($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      value = "",
      id,
      readonly = false,
      focus = false,
      select = false,
      type = "text",
      placeholder = "",
      disabled = false,
      error = false,
      title = "",
      tooltip,
      css = "",
      icon = "",
      clear = false,
      onchange
    } = $$props;
    const inputId = getInputId(id);
    let cssString = derived(() => icon && css.indexOf("wx-icon-left") === -1 ? "wx-icon-right " + css : css);
    let hasLeftIcon = derived(() => icon && css.indexOf("wx-icon-left") !== -1);
    $$renderer2.push(`<div${attr_class(`wx-text ${stringify(cssString())}`, "svelte-1up3qrx", {
      "wx-error": error,
      "wx-disabled": disabled,
      "wx-clear": clear
    })}${attr("data-tooltip-text", tooltip)}>`);
    if (type == "password") {
      $$renderer2.push(`<!--[0--><input${attr("value", value)}${attr("id", inputId)}${attr("readonly", readonly, true)}${attr("disabled", disabled, true)}${attr("placeholder", placeholder)} type="password"${attr("title", title)} class="svelte-1up3qrx"/>`);
    } else if (type == "number") {
      $$renderer2.push(`<!--[1--><input${attr("value", value)}${attr("id", inputId)}${attr("readonly", readonly, true)}${attr("disabled", disabled, true)}${attr("placeholder", placeholder)} type="number"${attr("title", title)} class="svelte-1up3qrx"/>`);
    } else {
      $$renderer2.push(`<!--[-1--><input${attr("value", value)}${attr("id", inputId)}${attr("readonly", readonly, true)}${attr("disabled", disabled, true)}${attr("placeholder", placeholder)}${attr("title", title)} class="svelte-1up3qrx"/>`);
    }
    $$renderer2.push(`<!--]--> `);
    if (clear && !disabled && value) {
      $$renderer2.push(`<!--[0--><i class="wx-icon wxi-close svelte-1up3qrx"></i> `);
      if (hasLeftIcon()) {
        $$renderer2.push(`<!--[0--><i${attr_class(`wx-icon ${stringify(icon)}`, "svelte-1up3qrx")}></i>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]-->`);
    } else if (icon) {
      $$renderer2.push(`<!--[1--><i${attr_class(`wx-icon ${stringify(icon)}`, "svelte-1up3qrx")}></i>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div>`);
    bind_props($$props, { value });
  });
}
function Field($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      label = "",
      position = "",
      width = "",
      error = false,
      type = "",
      required = false,
      id,
      css = "",
      children
    } = $$props;
    const inputId = id === void 0 ? uid() : id;
    setContext("wx-input-id", inputId);
    $$renderer2.push(`<div${attr_class(`wx-field wx-${stringify(position)} ${stringify(css)}`, "svelte-234ckw", { "wx-error": error, "wx-required": required })}${attr_style(width ? `width: ${width}` : "")}>`);
    if (label) {
      $$renderer2.push("<!--[0-->");
      if (inputId) {
        $$renderer2.push(`<!--[0--><label class="wx-label svelte-234ckw"${attr("for", inputId)}>${escape_html(label)}</label>`);
      } else {
        $$renderer2.push(`<!--[-1--><div class="wx-label svelte-234ckw">${escape_html(label)}</div>`);
      }
      $$renderer2.push(`<!--]-->`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div${attr_class(`wx-field-control wx-${stringify(type)}`, "svelte-234ckw")}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></div></div>`);
  });
}
setEnv(env);
const handlers$1 = {};
function registerEditorItem(type, handler) {
  handlers$1[type] = handler;
}
const handlers = {};
function registerToolbarItem(type, handler) {
  handlers[type] = handler;
}
function Separator($$renderer, $$props) {
  let { menu = false } = $$props;
  $$renderer.push(`<div${attr_class(`wx-separator${menu ? "-menu" : ""}`, "svelte-1cdagfa")}> </div>`);
}
function Spacer($$renderer) {
  $$renderer.push(`<div class="wx-spacer svelte-krf5sf"></div>`);
}
function Button_1($$renderer, $$props) {
  let {
    icon,
    title,
    text = "",
    tooltip,
    css,
    type,
    disabled,
    menu,
    onclick
  } = $$props;
  if (menu) {
    $$renderer.push(`<!--[0--><div class="wx-item svelte-1fht9vz"><i${attr_class(`${stringify(icon || "wxi-empty")} ${stringify(css || "")}`, "svelte-1fht9vz")}></i> ${escape_html(text)}</div>`);
  } else {
    $$renderer.push("<!--[-1-->");
    Button($$renderer, { icon, type, css, title, text, tooltip, disabled, onclick });
  }
  $$renderer.push(`<!--]-->`);
}
function Label($$renderer, $$props) {
  const { text, value, children } = $$props;
  if (children) {
    $$renderer.push(`<!--[0--><div class="wx-label svelte-12suhgh">`);
    children($$renderer);
    $$renderer.push(`<!----></div>`);
  } else {
    $$renderer.push(`<!--[-1--><div class="wx-label svelte-12suhgh">${escape_html(value || text)}</div>`);
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
    $$renderer.push(`<!--[0--><div class="wx-item svelte-uau686">`);
    if (icon) {
      $$renderer.push(`<!--[0--><i${attr_class(`${stringify(icon)} ${stringify(css)}`, "svelte-uau686")}></i>`);
    } else {
      $$renderer.push("<!--[-1-->");
    }
    $$renderer.push(`<!--]--> ${escape_html(text)}</div>`);
  } else {
    $$renderer.push("<!--[-1-->");
    Button($$renderer, { icon, type, css, disabled, title, tooltip, onclick });
  }
  $$renderer.push(`<!--]-->`);
}
function Item($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { id = "", text = "", css = "", icon = "", onclick } = $$props;
    $$renderer2.push(`<div${attr_class(`wx-label ${stringify(css)}`, "svelte-1cifv00")}>`);
    if (icon) {
      $$renderer2.push(`<!--[0--><i${attr_class(clsx(icon), "svelte-1cifv00")}></i>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> ${escape_html(text)}</div>`);
  });
}
registerToolbarItem("button", Button_1);
registerToolbarItem("separator", Separator);
registerToolbarItem("spacer", Spacer);
registerToolbarItem("label", Label);
registerToolbarItem("item", Item);
registerToolbarItem("icon", Icon);
function ReadOnly($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const _ = getContext("wx-i18n").getGroup("editor");
    let { value, options, label } = $$props;
    let text = derived(() => {
      let text2 = value;
      if (typeof value === "boolean") {
        text2 = value ? _("Yes") : _("No");
      }
      if (options) {
        const option = options.find((o) => o.id === value);
        if (option) {
          text2 = option.label;
        }
      }
      return text2;
    });
    if (text() || text() === 0) {
      $$renderer2.push("<!--[0-->");
      Field($$renderer2, {
        label,
        children: ($$renderer3) => {
          $$renderer3.push(`<!---->${escape_html(text())}`);
        }
      });
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
  });
}
function Section($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { key, label, activeSection, onclick } = $$props;
    $$renderer2.push(`<div${attr_class("wx-section svelte-tp8pkw", void 0, { "wx-section-active": activeSection })}><h3>${escape_html(label)}</h3> <i${attr_class(`wxi-angle-${activeSection ? "down" : "right"} wx-icon`, "svelte-tp8pkw")}></i></div>`);
  });
}
registerEditorItem("text", Text);
registerEditorItem("textarea", TextArea);
registerEditorItem("checkbox", Checkbox);
registerEditorItem("readonly", ReadOnly);
registerEditorItem("section", Section);
setEnv(env);
(/* @__PURE__ */ new Date()).valueOf();
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    $$renderer2.push(`<h1>${escape_html(t("nav.requisitions"))}</h1> `);
    {
      $$renderer2.push(`<!--[1--><p>${escape_html(t("common.loading"))}</p>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
