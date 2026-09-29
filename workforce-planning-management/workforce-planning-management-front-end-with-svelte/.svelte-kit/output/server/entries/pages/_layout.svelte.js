import { a as attributes, b as bind_props, e as element, d as derived, c as clsx, f as attr, g as ensure_array_like, s as spread_props, h as attr_class } from "../../chunks/index2.js";
import { p as page } from "../../chunks/index.js";
import { t, L as LOCALE_LABELS, i as i18n, a as LOCALES } from "../../chunks/i18n.svelte.js";
import { e as escape_html } from "../../chunks/escaping.js";
function IconButton($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className = "",
      baseClass = "icon-button",
      label,
      type = "button",
      disabled = false,
      pressed = void 0,
      onclick = void 0,
      ref = void 0,
      children,
      $$slots,
      $$events,
      ...restProps
      /** Base class token, replacing "icon-button" outright (not appended). */
      /** Accessible label (REQUIRED for icon-only buttons) */
      /** HTML button type */
      /** Whether the button is disabled */
      /** Toggle button pressed state */
      /** Click handler */
      /** The rendered button element. Bindable. */
      /** Icon content */
    } = $$props;
    $$renderer2.push(`<button${attributes({
      class: `${baseClass} ${className}`,
      type,
      disabled,
      "aria-label": label,
      "aria-pressed": pressed,
      ...restProps
    })}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></button>`);
    bind_props($$props, { ref });
  });
}
function Listbox($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className = "",
      baseClass = "listbox",
      as = "div",
      label,
      children,
      navigation = "roving-focus",
      ref = void 0,
      activeIndex = -1,
      clamp = false,
      typeahead = false,
      pageSize = 10,
      onActivate,
      onEscape,
      onTabOut,
      $$slots,
      $$events,
      ...restProps
      /** Base class token, replacing "listbox" outright (not appended). */
      /** Root element tag. Default "div" (unchanged). A consumer whose spec
       * requires e.g. a `<ul>` root sets `as="ul"`. */
      /** Accessible label. */
      /** Option elements. */
      /** Navigation/focus model. Default "roving-focus" (unchanged legacy behaviour). */
      /** The rendered root element. Bindable. */
      /** Virtual cursor position (active-descendant mode). Bindable, -1 = none. */
      /** Arrow keys clamp instead of wrap (active-descendant mode). */
      /** Printable-character typeahead (active-descendant mode). */
      /** PageUp/PageDown step size (active-descendant mode). */
      /** Enter/Space on the active option (active-descendant mode). */
      /** Escape pressed (active-descendant mode). */
      /** Tab pressed, called before the key is processed (active-descendant mode). */
    } = $$props;
    function options() {
      return ref ? Array.from(ref.querySelectorAll("[role='option']")) : [];
    }
    const activeId = derived(() => {
      if (navigation !== "active-descendant" || activeIndex < 0) return void 0;
      return options()[activeIndex]?.id || void 0;
    });
    element(
      $$renderer2,
      as,
      () => {
        $$renderer2.push(`${attributes({
          class: `${baseClass} ${className}`,
          role: "listbox",
          "aria-label": label,
          tabindex: navigation === "active-descendant" ? -1 : void 0,
          "aria-activedescendant": activeId(),
          ...restProps
        })}`);
      },
      () => {
        children?.($$renderer2);
        $$renderer2.push(`<!---->`);
      }
    );
    bind_props($$props, { ref, activeIndex });
  });
}
function themeName(theme) {
  return theme.split("-").map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join(" ");
}
let uid$3 = 0;
function nextThemePickerId() {
  uid$3 += 1;
  return `theme-picker-${uid$3}`;
}
function ThemePicker($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className = "",
      label,
      themesUrl,
      themes,
      value = "",
      defaultValue,
      storageKey,
      detectFromSystem = false,
      name = "theme",
      extension = ".css",
      target,
      themeLabels = {},
      children,
      onChange,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    const baseId = nextThemePickerId();
    const listId = `${baseId}-list`;
    const optionId = (i) => `${baseId}-option-${i}`;
    let open = false;
    let activeIndex = -1;
    let buttonEl = void 0;
    let listEl = void 0;
    function labelFor(theme) {
      if (theme in themeLabels) return themeLabels[theme];
      return themeName(theme);
    }
    function setTheme(slug) {
      value = slug;
    }
    function openList(startIndex) {
      const selected = themes.indexOf(value);
      activeIndex = themes.length === 0 ? -1 : startIndex ?? (selected >= 0 ? selected : 0);
      open = true;
      queueMicrotask(() => {
        listEl?.focus({ preventScroll: true });
        scrollActiveIntoView();
      });
    }
    function closeList(refocus = true) {
      if (!open) return;
      open = false;
      activeIndex = -1;
      if (refocus) queueMicrotask(() => buttonEl?.focus({ preventScroll: true }));
    }
    function choose(index) {
      const slug = themes[index];
      if (slug) setTheme(slug);
      closeList();
    }
    function scrollActiveIntoView() {
      if (activeIndex < 0 || !listEl) return;
      const el = document.getElementById(optionId(activeIndex));
      el?.scrollIntoView?.({ block: "nearest" });
    }
    function handleTabOut() {
      buttonEl?.focus?.({ preventScroll: true });
      closeList(false);
    }
    function onButtonKeydown(event) {
      switch (event.key) {
        case "ArrowDown":
        case "Enter":
        case " ":
          event.preventDefault();
          openList();
          break;
        case "ArrowUp":
          event.preventDefault();
          openList(themes.length - 1);
          break;
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div${attributes({
        class: clsx(`theme-picker ${className}`.trim()),
        ...restProps
      })}><input type="hidden"${attr("name", name)}${attr("value", value)}/> `);
      IconButton($$renderer3, {
        baseClass: "theme-picker-button",
        label,
        "aria-haspopup": "listbox",
        "aria-expanded": open,
        "aria-controls": listId,
        onclick: () => open ? closeList() : openList(),
        onkeydown: onButtonKeydown,
        get ref() {
          return buttonEl;
        },
        set ref($$value) {
          buttonEl = $$value;
          $$settled = false;
        },
        children: ($$renderer4) => {
          if (children) {
            $$renderer4.push("<!--[0-->");
            children($$renderer4, { value: value ?? "", open, labelFor });
            $$renderer4.push(`<!---->`);
          } else {
            $$renderer4.push(`<!--[-1--><svg class="theme-picker-icon" viewBox="0 0 16 16" width="1.05rem" height="1.05rem" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"></circle><path d="M8 2a6 6 0 0 1 0 12z" fill="currentColor" stroke="none"></path></svg>`);
          }
          $$renderer4.push(`<!--]-->`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> `);
      Listbox($$renderer3, {
        as: "ul",
        baseClass: "theme-picker-list",
        id: listId,
        label,
        navigation: "active-descendant",
        clamp: true,
        typeahead: true,
        pageSize: 10,
        hidden: !open,
        onActivate: choose,
        onEscape: () => closeList(),
        onTabOut: handleTabOut,
        get ref() {
          return listEl;
        },
        set ref($$value) {
          listEl = $$value;
          $$settled = false;
        },
        get activeIndex() {
          return activeIndex;
        },
        set activeIndex($$value) {
          activeIndex = $$value;
          $$settled = false;
        },
        children: ($$renderer4) => {
          $$renderer4.push(`<!--[-->`);
          const each_array = ensure_array_like(themes);
          for (let i = 0, $$length = each_array.length; i < $$length; i++) {
            let theme = each_array[i];
            $$renderer4.push(`<li class="theme-picker-option"${attr("id", optionId(i))} role="option"${attr("aria-selected", theme === value)}${attr("data-active", i === activeIndex ? "" : void 0)}>${escape_html(labelFor(theme))}</li>`);
          }
          $$renderer4.push(`<!--]-->`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----></div>`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
    bind_props($$props, { value });
  });
}
const defaultLocaleLabels = {
  "af_NA": "Afrikaans (Namibia)",
  "af_ZA": "Afrikaans (South Africa)",
  "af": "Afrikaans",
  "ak_GH": "Akan (Ghana)",
  "ak": "Akan",
  "sq_AL": "Albanian (Albania)",
  "sq": "Albanian",
  "am_ET": "Amharic (Ethiopia)",
  "am": "Amharic",
  "ar_DZ": "Arabic (Algeria)",
  "ar_BH": "Arabic (Bahrain)",
  "ar_EG": "Arabic (Egypt)",
  "ar_IQ": "Arabic (Iraq)",
  "ar_JO": "Arabic (Jordan)",
  "ar_KW": "Arabic (Kuwait)",
  "ar_LB": "Arabic (Lebanon)",
  "ar_LY": "Arabic (Libya)",
  "ar_MA": "Arabic (Morocco)",
  "ar_OM": "Arabic (Oman)",
  "ar_QA": "Arabic (Qatar)",
  "ar_SA": "Arabic (Saudi Arabia)",
  "ar_SD": "Arabic (Sudan)",
  "ar_SY": "Arabic (Syria)",
  "ar_TN": "Arabic (Tunisia)",
  "ar_AE": "Arabic (United Arab Emirates)",
  "ar_YE": "Arabic (Yemen)",
  "ar": "Arabic",
  "hy_AM": "Armenian (Armenia)",
  "hy": "Armenian",
  "as_IN": "Assamese (India)",
  "as": "Assamese",
  "asa_TZ": "Asu (Tanzania)",
  "asa": "Asu",
  "az_Cyrl": "Azerbaijani (Cyrillic)",
  "az_Cyrl_AZ": "Azerbaijani (Cyrillic, Azerbaijan)",
  "az_Latn": "Azerbaijani (Latin)",
  "az_Latn_AZ": "Azerbaijani (Latin, Azerbaijan)",
  "az": "Azerbaijani",
  "bm_ML": "Bambara (Mali)",
  "bm": "Bambara",
  "eu_ES": "Basque (Spain)",
  "eu": "Basque",
  "be_BY": "Belarusian (Belarus)",
  "be": "Belarusian",
  "bem_ZM": "Bemba (Zambia)",
  "bem": "Bemba",
  "bez_TZ": "Bena (Tanzania)",
  "bez": "Bena",
  "bn_BD": "Bengali (Bangladesh)",
  "bn_IN": "Bengali (India)",
  "bn": "Bengali",
  "bs_BA": "Bosnian (Bosnia and Herzegovina)",
  "bs": "Bosnian",
  "bg_BG": "Bulgarian (Bulgaria)",
  "bg": "Bulgarian",
  "my_MM": "Burmese (Myanmar [Burma])",
  "my": "Burmese",
  "yue_Hant_HK": "Cantonese (Traditional, Hong Kong SAR China)",
  "ca_ES": "Catalan (Spain)",
  "ca": "Catalan",
  "tzm_Latn": "Central Morocco Tamazight (Latin)",
  "tzm_Latn_MA": "Central Morocco Tamazight (Latin, Morocco)",
  "tzm": "Central Morocco Tamazight",
  "chr_US": "Cherokee (United States)",
  "chr": "Cherokee",
  "cgg_UG": "Chiga (Uganda)",
  "cgg": "Chiga",
  "zh_Hans": "Chinese (Simplified Han)",
  "zh_Hans_CN": "Chinese (Simplified Han, China)",
  "zh_Hans_HK": "Chinese (Simplified Han, Hong Kong SAR China)",
  "zh_Hans_MO": "Chinese (Simplified Han, Macau SAR China)",
  "zh_Hans_SG": "Chinese (Simplified Han, Singapore)",
  "zh_Hant": "Chinese (Traditional Han)",
  "zh_Hant_HK": "Chinese (Traditional Han, Hong Kong SAR China)",
  "zh_Hant_MO": "Chinese (Traditional Han, Macau SAR China)",
  "zh_Hant_TW": "Chinese (Traditional Han, Taiwan)",
  "zh": "Chinese",
  "kw_GB": "Cornish (United Kingdom)",
  "kw": "Cornish",
  "hr_HR": "Croatian (Croatia)",
  "hr": "Croatian",
  "cs_CZ": "Czech (Czech Republic)",
  "cs": "Czech",
  "da_DK": "Danish (Denmark)",
  "da": "Danish",
  "nl_BE": "Dutch (Belgium)",
  "nl_NL": "Dutch (Netherlands)",
  "nl": "Dutch",
  "ebu_KE": "Embu (Kenya)",
  "ebu": "Embu",
  "en_AS": "English (American Samoa)",
  "en_AU": "English (Australia)",
  "en_BE": "English (Belgium)",
  "en_BZ": "English (Belize)",
  "en_BW": "English (Botswana)",
  "en_CA": "English (Canada)",
  "en_GU": "English (Guam)",
  "en_HK": "English (Hong Kong SAR China)",
  "en_IN": "English (India)",
  "en_IE": "English (Ireland)",
  "en_IL": "English (Israel)",
  "en_JM": "English (Jamaica)",
  "en_MT": "English (Malta)",
  "en_MH": "English (Marshall Islands)",
  "en_MU": "English (Mauritius)",
  "en_NA": "English (Namibia)",
  "en_NZ": "English (New Zealand)",
  "en_MP": "English (Northern Mariana Islands)",
  "en_PK": "English (Pakistan)",
  "en_PH": "English (Philippines)",
  "en_SG": "English (Singapore)",
  "en_ZA": "English (South Africa)",
  "en_TT": "English (Trinidad and Tobago)",
  "en_UM": "English (U.S. Minor Outlying Islands)",
  "en_VI": "English (U.S. Virgin Islands)",
  "en_GB": "English (United Kingdom)",
  "en_US": "English (United States)",
  "en_ZW": "English (Zimbabwe)",
  "en": "English",
  "eo": "Esperanto",
  "et_EE": "Estonian (Estonia)",
  "et": "Estonian",
  "ee_GH": "Ewe (Ghana)",
  "ee_TG": "Ewe (Togo)",
  "ee": "Ewe",
  "fo_FO": "Faroese (Faroe Islands)",
  "fo": "Faroese",
  "fil_PH": "Filipino (Philippines)",
  "fil": "Filipino",
  "fi_FI": "Finnish (Finland)",
  "fi": "Finnish",
  "fr_BE": "French (Belgium)",
  "fr_BJ": "French (Benin)",
  "fr_BF": "French (Burkina Faso)",
  "fr_BI": "French (Burundi)",
  "fr_CM": "French (Cameroon)",
  "fr_CA": "French (Canada)",
  "fr_CF": "French (Central African Republic)",
  "fr_TD": "French (Chad)",
  "fr_KM": "French (Comoros)",
  "fr_CG": "French (Congo - Brazzaville)",
  "fr_CD": "French (Congo - Kinshasa)",
  "fr_CI": "French (Côte d’Ivoire)",
  "fr_DJ": "French (Djibouti)",
  "fr_GQ": "French (Equatorial Guinea)",
  "fr_FR": "French (France)",
  "fr_GA": "French (Gabon)",
  "fr_GP": "French (Guadeloupe)",
  "fr_GN": "French (Guinea)",
  "fr_LU": "French (Luxembourg)",
  "fr_MG": "French (Madagascar)",
  "fr_ML": "French (Mali)",
  "fr_MQ": "French (Martinique)",
  "fr_MC": "French (Monaco)",
  "fr_NE": "French (Niger)",
  "fr_RW": "French (Rwanda)",
  "fr_RE": "French (Réunion)",
  "fr_BL": "French (Saint Barthélemy)",
  "fr_MF": "French (Saint Martin)",
  "fr_SN": "French (Senegal)",
  "fr_CH": "French (Switzerland)",
  "fr_TG": "French (Togo)",
  "fr": "French",
  "ff_SN": "Fulah (Senegal)",
  "ff": "Fulah",
  "gl_ES": "Galician (Spain)",
  "gl": "Galician",
  "lg_UG": "Ganda (Uganda)",
  "lg": "Ganda",
  "ka_GE": "Georgian (Georgia)",
  "ka": "Georgian",
  "de_AT": "German (Austria)",
  "de_BE": "German (Belgium)",
  "de_DE": "German (Germany)",
  "de_LI": "German (Liechtenstein)",
  "de_LU": "German (Luxembourg)",
  "de_CH": "German (Switzerland)",
  "de": "German",
  "el_CY": "Greek (Cyprus)",
  "el_GR": "Greek (Greece)",
  "el": "Greek",
  "gu_IN": "Gujarati (India)",
  "gu": "Gujarati",
  "guz_KE": "Gusii (Kenya)",
  "guz": "Gusii",
  "ha_Latn": "Hausa (Latin)",
  "ha_Latn_GH": "Hausa (Latin, Ghana)",
  "ha_Latn_NE": "Hausa (Latin, Niger)",
  "ha_Latn_NG": "Hausa (Latin, Nigeria)",
  "ha": "Hausa",
  "haw_US": "Hawaiian (United States)",
  "haw": "Hawaiian",
  "he_IL": "Hebrew (Israel)",
  "he": "Hebrew",
  "hi_IN": "Hindi (India)",
  "hi": "Hindi",
  "hu_HU": "Hungarian (Hungary)",
  "hu": "Hungarian",
  "is_IS": "Icelandic (Iceland)",
  "is": "Icelandic",
  "ig_NG": "Igbo (Nigeria)",
  "ig": "Igbo",
  "id_ID": "Indonesian (Indonesia)",
  "id": "Indonesian",
  "ga_IE": "Irish (Ireland)",
  "ga": "Irish",
  "it_IT": "Italian (Italy)",
  "it_CH": "Italian (Switzerland)",
  "it": "Italian",
  "ja_JP": "Japanese (Japan)",
  "ja": "Japanese",
  "kea_CV": "Kabuverdianu (Cape Verde)",
  "kea": "Kabuverdianu",
  "kab_DZ": "Kabyle (Algeria)",
  "kab": "Kabyle",
  "kl_GL": "Kalaallisut (Greenland)",
  "kl": "Kalaallisut",
  "kln_KE": "Kalenjin (Kenya)",
  "kln": "Kalenjin",
  "kam_KE": "Kamba (Kenya)",
  "kam": "Kamba",
  "kn_IN": "Kannada (India)",
  "kn": "Kannada",
  "kk_Cyrl": "Kazakh (Cyrillic)",
  "kk_Cyrl_KZ": "Kazakh (Cyrillic, Kazakhstan)",
  "kk": "Kazakh",
  "km_KH": "Khmer (Cambodia)",
  "km": "Khmer",
  "ki_KE": "Kikuyu (Kenya)",
  "ki": "Kikuyu",
  "rw_RW": "Kinyarwanda (Rwanda)",
  "rw": "Kinyarwanda",
  "kok_IN": "Konkani (India)",
  "kok": "Konkani",
  "ko_KR": "Korean (South Korea)",
  "ko": "Korean",
  "khq_ML": "Koyra Chiini (Mali)",
  "khq": "Koyra Chiini",
  "ses_ML": "Koyraboro Senni (Mali)",
  "ses": "Koyraboro Senni",
  "lag_TZ": "Langi (Tanzania)",
  "lag": "Langi",
  "lv_LV": "Latvian (Latvia)",
  "lv": "Latvian",
  "lt_LT": "Lithuanian (Lithuania)",
  "lt": "Lithuanian",
  "luo_KE": "Luo (Kenya)",
  "luo": "Luo",
  "luy_KE": "Luyia (Kenya)",
  "luy": "Luyia",
  "mk_MK": "Macedonian (Macedonia)",
  "mk": "Macedonian",
  "jmc_TZ": "Machame (Tanzania)",
  "jmc": "Machame",
  "kde_TZ": "Makonde (Tanzania)",
  "kde": "Makonde",
  "mg_MG": "Malagasy (Madagascar)",
  "mg": "Malagasy",
  "ms_BN": "Malay (Brunei)",
  "ms_MY": "Malay (Malaysia)",
  "ms": "Malay",
  "ml_IN": "Malayalam (India)",
  "ml": "Malayalam",
  "mt_MT": "Maltese (Malta)",
  "mt": "Maltese",
  "gv_GB": "Manx (United Kingdom)",
  "gv": "Manx",
  "mr_IN": "Marathi (India)",
  "mr": "Marathi",
  "mas_KE": "Masai (Kenya)",
  "mas_TZ": "Masai (Tanzania)",
  "mas": "Masai",
  "mer_KE": "Meru (Kenya)",
  "mer": "Meru",
  "mfe_MU": "Morisyen (Mauritius)",
  "mfe": "Morisyen",
  "naq_NA": "Nama (Namibia)",
  "naq": "Nama",
  "ne_IN": "Nepali (India)",
  "ne_NP": "Nepali (Nepal)",
  "ne": "Nepali",
  "nd_ZW": "North Ndebele (Zimbabwe)",
  "nd": "North Ndebele",
  "nb_NO": "Norwegian Bokmål (Norway)",
  "nb": "Norwegian Bokmål",
  "nn_NO": "Norwegian Nynorsk (Norway)",
  "nn": "Norwegian Nynorsk",
  "nyn_UG": "Nyankole (Uganda)",
  "nyn": "Nyankole",
  "or_IN": "Oriya (India)",
  "or": "Oriya",
  "om_ET": "Oromo (Ethiopia)",
  "om_KE": "Oromo (Kenya)",
  "om": "Oromo",
  "ps_AF": "Pashto (Afghanistan)",
  "ps": "Pashto",
  "fa_AF": "Persian (Afghanistan)",
  "fa_IR": "Persian (Iran)",
  "fa": "Persian",
  "pl_PL": "Polish (Poland)",
  "pl": "Polish",
  "pt_BR": "Portuguese (Brazil)",
  "pt_GW": "Portuguese (Guinea-Bissau)",
  "pt_MZ": "Portuguese (Mozambique)",
  "pt_PT": "Portuguese (Portugal)",
  "pt": "Portuguese",
  "pa_Arab": "Punjabi (Arabic)",
  "pa_Arab_PK": "Punjabi (Arabic, Pakistan)",
  "pa_Guru": "Punjabi (Gurmukhi)",
  "pa_Guru_IN": "Punjabi (Gurmukhi, India)",
  "pa": "Punjabi",
  "ro_MD": "Romanian (Moldova)",
  "ro_RO": "Romanian (Romania)",
  "ro": "Romanian",
  "rm_CH": "Romansh (Switzerland)",
  "rm": "Romansh",
  "rof_TZ": "Rombo (Tanzania)",
  "rof": "Rombo",
  "ru_MD": "Russian (Moldova)",
  "ru_RU": "Russian (Russia)",
  "ru_UA": "Russian (Ukraine)",
  "ru": "Russian",
  "rwk_TZ": "Rwa (Tanzania)",
  "rwk": "Rwa",
  "saq_KE": "Samburu (Kenya)",
  "saq": "Samburu",
  "sg_CF": "Sango (Central African Republic)",
  "sg": "Sango",
  "seh_MZ": "Sena (Mozambique)",
  "seh": "Sena",
  "sr_Cyrl": "Serbian (Cyrillic)",
  "sr_Cyrl_BA": "Serbian (Cyrillic, Bosnia and Herzegovina)",
  "sr_Cyrl_ME": "Serbian (Cyrillic, Montenegro)",
  "sr_Cyrl_RS": "Serbian (Cyrillic, Serbia)",
  "sr_Latn": "Serbian (Latin)",
  "sr_Latn_BA": "Serbian (Latin, Bosnia and Herzegovina)",
  "sr_Latn_ME": "Serbian (Latin, Montenegro)",
  "sr_Latn_RS": "Serbian (Latin, Serbia)",
  "sr": "Serbian",
  "sn_ZW": "Shona (Zimbabwe)",
  "sn": "Shona",
  "ii_CN": "Sichuan Yi (China)",
  "ii": "Sichuan Yi",
  "si_LK": "Sinhala (Sri Lanka)",
  "si": "Sinhala",
  "sk_SK": "Slovak (Slovakia)",
  "sk": "Slovak",
  "sl_SI": "Slovenian (Slovenia)",
  "sl": "Slovenian",
  "xog_UG": "Soga (Uganda)",
  "xog": "Soga",
  "so_DJ": "Somali (Djibouti)",
  "so_ET": "Somali (Ethiopia)",
  "so_KE": "Somali (Kenya)",
  "so_SO": "Somali (Somalia)",
  "so": "Somali",
  "es_AR": "Spanish (Argentina)",
  "es_BO": "Spanish (Bolivia)",
  "es_CL": "Spanish (Chile)",
  "es_CO": "Spanish (Colombia)",
  "es_CR": "Spanish (Costa Rica)",
  "es_DO": "Spanish (Dominican Republic)",
  "es_EC": "Spanish (Ecuador)",
  "es_SV": "Spanish (El Salvador)",
  "es_GQ": "Spanish (Equatorial Guinea)",
  "es_GT": "Spanish (Guatemala)",
  "es_HN": "Spanish (Honduras)",
  "es_419": "Spanish (Latin America)",
  "es_MX": "Spanish (Mexico)",
  "es_NI": "Spanish (Nicaragua)",
  "es_PA": "Spanish (Panama)",
  "es_PY": "Spanish (Paraguay)",
  "es_PE": "Spanish (Peru)",
  "es_PR": "Spanish (Puerto Rico)",
  "es_ES": "Spanish (Spain)",
  "es_US": "Spanish (United States)",
  "es_UY": "Spanish (Uruguay)",
  "es_VE": "Spanish (Venezuela)",
  "es": "Spanish",
  "sw_KE": "Swahili (Kenya)",
  "sw_TZ": "Swahili (Tanzania)",
  "sw": "Swahili",
  "sv_FI": "Swedish (Finland)",
  "sv_SE": "Swedish (Sweden)",
  "sv": "Swedish",
  "gsw_CH": "Swiss German (Switzerland)",
  "gsw": "Swiss German",
  "shi_Latn": "Tachelhit (Latin)",
  "shi_Latn_MA": "Tachelhit (Latin, Morocco)",
  "shi_Tfng": "Tachelhit (Tifinagh)",
  "shi_Tfng_MA": "Tachelhit (Tifinagh, Morocco)",
  "shi": "Tachelhit",
  "dav_KE": "Taita (Kenya)",
  "dav": "Taita",
  "ta_IN": "Tamil (India)",
  "ta_LK": "Tamil (Sri Lanka)",
  "ta": "Tamil",
  "te_IN": "Telugu (India)",
  "te": "Telugu",
  "teo_KE": "Teso (Kenya)",
  "teo_UG": "Teso (Uganda)",
  "teo": "Teso",
  "th_TH": "Thai (Thailand)",
  "th": "Thai",
  "bo_CN": "Tibetan (China)",
  "bo_IN": "Tibetan (India)",
  "bo": "Tibetan",
  "ti_ER": "Tigrinya (Eritrea)",
  "ti_ET": "Tigrinya (Ethiopia)",
  "ti": "Tigrinya",
  "to_TO": "Tonga (Tonga)",
  "to": "Tonga",
  "tr_TR": "Turkish (Turkey)",
  "tr": "Turkish",
  "uk_UA": "Ukrainian (Ukraine)",
  "uk": "Ukrainian",
  "ur_IN": "Urdu (India)",
  "ur_PK": "Urdu (Pakistan)",
  "ur": "Urdu",
  "uz_Arab": "Uzbek (Arabic)",
  "uz_Arab_AF": "Uzbek (Arabic, Afghanistan)",
  "uz_Cyrl": "Uzbek (Cyrillic)",
  "uz_Cyrl_UZ": "Uzbek (Cyrillic, Uzbekistan)",
  "uz_Latn": "Uzbek (Latin)",
  "uz_Latn_UZ": "Uzbek (Latin, Uzbekistan)",
  "uz": "Uzbek",
  "vi_VN": "Vietnamese (Vietnam)",
  "vi": "Vietnamese",
  "vun_TZ": "Vunjo (Tanzania)",
  "vun": "Vunjo",
  "cy_GB": "Welsh (United Kingdom)",
  "cy": "Welsh",
  "yo_NG": "Yoruba (Nigeria)",
  "yo": "Yoruba",
  "zu_ZA": "Zulu (South Africa)",
  "zu": "Zulu"
};
function bcp47LocaleTag(locale) {
  return locale.replace(/_/g, "-");
}
function localeEndonym(locale) {
  try {
    const tag = bcp47LocaleTag(locale);
    const dn = new Intl.DisplayNames([tag], { type: "language" });
    const found = dn.of(tag) ?? "";
    return found && found.toLowerCase() !== tag.toLowerCase() ? found : "";
  } catch {
    return "";
  }
}
function intlDisplayName(locale) {
  try {
    const env = typeof navigator !== "undefined" && navigator.language ? navigator.language : "en";
    const dn = new Intl.DisplayNames([env], { type: "language" });
    return dn.of(bcp47LocaleTag(locale)) ?? "";
  } catch {
    return "";
  }
}
let uid$2 = 0;
function nextLocalePickerId() {
  uid$2 += 1;
  return `locale-picker-${uid$2}`;
}
function LocalePicker($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className = "",
      label,
      locales,
      value = "",
      defaultValue,
      storageKey,
      detectFromNavigator = false,
      name = "locale",
      target,
      applyDir = true,
      localeLabels = {},
      children,
      onChange,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    const baseId = nextLocalePickerId();
    const listId = `${baseId}-list`;
    const optionId = (i) => `${baseId}-option-${i}`;
    let open = false;
    let activeIndex = -1;
    let buttonEl = void 0;
    let listEl = void 0;
    function labelFor(locale) {
      if (locale in localeLabels) return localeLabels[locale];
      const endonym = localeEndonym(locale);
      if (endonym) return endonym;
      if (locale in defaultLocaleLabels) return defaultLocaleLabels[locale];
      const intl = intlDisplayName(locale);
      if (intl) return intl;
      return locale;
    }
    function optionLang(locale) {
      if (locale in localeLabels) return void 0;
      return localeEndonym(locale) ? bcp47LocaleTag(locale) : void 0;
    }
    function setLocale(code) {
      value = code;
    }
    function openList(startIndex) {
      const selected = locales.indexOf(value);
      activeIndex = locales.length === 0 ? -1 : startIndex ?? (selected >= 0 ? selected : 0);
      open = true;
      queueMicrotask(() => {
        listEl?.focus({ preventScroll: true });
        scrollActiveIntoView();
      });
    }
    function closeList(refocus = true) {
      if (!open) return;
      open = false;
      activeIndex = -1;
      if (refocus) queueMicrotask(() => buttonEl?.focus({ preventScroll: true }));
    }
    function choose(index) {
      const code = locales[index];
      if (code) setLocale(code);
      closeList();
    }
    function scrollActiveIntoView() {
      if (activeIndex < 0 || !listEl) return;
      const el = document.getElementById(optionId(activeIndex));
      el?.scrollIntoView?.({ block: "nearest" });
    }
    function handleTabOut() {
      buttonEl?.focus?.({ preventScroll: true });
      closeList(false);
    }
    function onButtonKeydown(event) {
      switch (event.key) {
        case "ArrowDown":
        case "Enter":
        case " ":
          event.preventDefault();
          openList();
          break;
        case "ArrowUp":
          event.preventDefault();
          openList(locales.length - 1);
          break;
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div${attributes({
        class: clsx(`locale-picker ${className}`.trim()),
        ...restProps
      })}><input type="hidden"${attr("name", name)}${attr("value", value)}/> `);
      IconButton($$renderer3, {
        baseClass: "locale-picker-button",
        label,
        "aria-haspopup": "listbox",
        "aria-expanded": open,
        "aria-controls": listId,
        onclick: () => open ? closeList() : openList(),
        onkeydown: onButtonKeydown,
        get ref() {
          return buttonEl;
        },
        set ref($$value) {
          buttonEl = $$value;
          $$settled = false;
        },
        children: ($$renderer4) => {
          if (children) {
            $$renderer4.push("<!--[0-->");
            children($$renderer4, { value: value ?? "", open, labelFor });
            $$renderer4.push(`<!---->`);
          } else {
            $$renderer4.push(`<!--[-1--><svg class="locale-picker-icon" viewBox="0 0 16 16" width="1.05rem" height="1.05rem" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"></circle><path d="M2 8h12"></path><path d="M8 2c2.2 0 4 2.7 4 6s-1.8 6-4 6-4-2.7-4-6 1.8-6 4-6z"></path></svg>`);
          }
          $$renderer4.push(`<!--]-->`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> `);
      Listbox($$renderer3, {
        as: "ul",
        baseClass: "locale-picker-list",
        id: listId,
        label,
        navigation: "active-descendant",
        clamp: true,
        typeahead: true,
        pageSize: 10,
        hidden: !open,
        onActivate: choose,
        onEscape: () => closeList(),
        onTabOut: handleTabOut,
        get ref() {
          return listEl;
        },
        set ref($$value) {
          listEl = $$value;
          $$settled = false;
        },
        get activeIndex() {
          return activeIndex;
        },
        set activeIndex($$value) {
          activeIndex = $$value;
          $$settled = false;
        },
        children: ($$renderer4) => {
          $$renderer4.push(`<!--[-->`);
          const each_array = ensure_array_like(locales);
          for (let i = 0, $$length = each_array.length; i < $$length; i++) {
            let locale = each_array[i];
            $$renderer4.push(`<li class="locale-picker-option"${attr("id", optionId(i))} role="option"${attr("aria-selected", locale === value)}${attr("data-active", i === activeIndex ? "" : void 0)}${attr("lang", optionLang(locale))}>${escape_html(labelFor(locale))}</li>`);
          }
          $$renderer4.push(`<!--]-->`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----></div>`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
    bind_props($$props, { value });
  });
}
function sizeName(size) {
  return size.split("-").map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join(" ");
}
let uid$1 = 0;
function nextTextSizePickerId() {
  uid$1 += 1;
  return `text-size-picker-${uid$1}`;
}
function TextSizePicker($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className = "",
      label,
      sizes,
      value = "",
      defaultValue,
      storageKey,
      name = "text-size",
      target,
      sizeLabels = {},
      children,
      onChange,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    const baseId = nextTextSizePickerId();
    const listId = `${baseId}-list`;
    const optionId = (i) => `${baseId}-option-${i}`;
    let open = false;
    let activeIndex = -1;
    let buttonEl = void 0;
    let listEl = void 0;
    function labelFor(size) {
      if (size in sizeLabels) return sizeLabels[size];
      return sizeName(size);
    }
    function setSize(slug) {
      value = slug;
    }
    function openList(startIndex) {
      const selected = sizes.indexOf(value);
      activeIndex = sizes.length === 0 ? -1 : startIndex ?? (selected >= 0 ? selected : 0);
      open = true;
      queueMicrotask(() => {
        listEl?.focus({ preventScroll: true });
        scrollActiveIntoView();
      });
    }
    function closeList(refocus = true) {
      if (!open) return;
      open = false;
      activeIndex = -1;
      if (refocus) queueMicrotask(() => buttonEl?.focus({ preventScroll: true }));
    }
    function choose(index) {
      const slug = sizes[index];
      if (slug) setSize(slug);
      closeList();
    }
    function scrollActiveIntoView() {
      if (activeIndex < 0 || !listEl) return;
      const el = document.getElementById(optionId(activeIndex));
      el?.scrollIntoView?.({ block: "nearest" });
    }
    function handleTabOut() {
      buttonEl?.focus?.({ preventScroll: true });
      closeList(false);
    }
    function onButtonKeydown(event) {
      switch (event.key) {
        case "ArrowDown":
        case "Enter":
        case " ":
          event.preventDefault();
          openList();
          break;
        case "ArrowUp":
          event.preventDefault();
          openList(sizes.length - 1);
          break;
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div${attributes({
        class: clsx(`text-size-picker ${className}`.trim()),
        ...restProps
      })}><input type="hidden"${attr("name", name)}${attr("value", value)}/> `);
      IconButton($$renderer3, {
        baseClass: "text-size-picker-button",
        label,
        "aria-haspopup": "listbox",
        "aria-expanded": open,
        "aria-controls": listId,
        onclick: () => open ? closeList() : openList(),
        onkeydown: onButtonKeydown,
        get ref() {
          return buttonEl;
        },
        set ref($$value) {
          buttonEl = $$value;
          $$settled = false;
        },
        children: ($$renderer4) => {
          if (children) {
            $$renderer4.push("<!--[0-->");
            children($$renderer4, { value: value ?? "", open, labelFor });
            $$renderer4.push(`<!---->`);
          } else {
            $$renderer4.push(`<!--[-1--><svg class="text-size-picker-icon" viewBox="0 0 16 16" width="1.05rem" height="1.05rem" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 13 7.2 3h1.6L12 13M5.4 9.5h5.2"></path></svg>`);
          }
          $$renderer4.push(`<!--]-->`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> `);
      Listbox($$renderer3, {
        as: "ul",
        baseClass: "text-size-picker-list",
        id: listId,
        label,
        navigation: "active-descendant",
        clamp: true,
        typeahead: true,
        pageSize: 10,
        hidden: !open,
        onActivate: choose,
        onEscape: () => closeList(),
        onTabOut: handleTabOut,
        get ref() {
          return listEl;
        },
        set ref($$value) {
          listEl = $$value;
          $$settled = false;
        },
        get activeIndex() {
          return activeIndex;
        },
        set activeIndex($$value) {
          activeIndex = $$value;
          $$settled = false;
        },
        children: ($$renderer4) => {
          $$renderer4.push(`<!--[-->`);
          const each_array = ensure_array_like(sizes);
          for (let i = 0, $$length = each_array.length; i < $$length; i++) {
            let size = each_array[i];
            $$renderer4.push(`<li class="text-size-picker-option"${attr("id", optionId(i))} role="option"${attr("aria-selected", size === value)}${attr("data-active", i === activeIndex ? "" : void 0)}>${escape_html(labelFor(size))}</li>`);
          }
          $$renderer4.push(`<!--]-->`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----></div>`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
    bind_props($$props, { value });
  });
}
function canShareNatively() {
  return typeof navigator !== "undefined" && typeof navigator.share === "function";
}
let uid = 0;
function nextSharePickerId() {
  uid += 1;
  return `share-picker-${uid}`;
}
function SharePicker($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className = "",
      label,
      targets = [],
      url,
      title = "",
      text = "",
      copyLabel,
      copiedLabel,
      copyFailedLabel,
      strategy = "auto",
      children,
      onShare,
      onCopy,
      onNativeShare,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    const baseId = nextSharePickerId();
    const listId = `${baseId}-list`;
    let open = false;
    let status = "";
    let buttonEl = void 0;
    function currentUrl() {
      if (url) return url;
      return typeof location !== "undefined" ? location.href : "";
    }
    function items() {
      return [];
    }
    function openList(focusLast = false) {
      open = true;
      status = "";
      queueMicrotask(() => {
        const all = items();
        (focusLast ? all[all.length - 1] : all[0])?.focus({ preventScroll: true });
      });
    }
    function closeList(refocus = true) {
      if (!open) return;
      open = false;
      if (refocus) queueMicrotask(() => buttonEl?.focus({ preventScroll: true }));
    }
    async function shareNatively() {
      if (!canShareNatively()) return false;
      const shareUrl = currentUrl();
      try {
        await navigator.share({ url: shareUrl, title, text });
        onNativeShare?.(shareUrl);
        return true;
      } catch {
        return true;
      }
    }
    async function onButtonClick() {
      if (open) {
        closeList();
        return;
      }
      if (strategy === "native" || strategy === "auto" && canShareNatively()) {
        if (await shareNatively()) return;
      }
      openList();
    }
    function onButtonKeydown(event) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        if (!open) openList();
        else items()[0]?.focus({ preventScroll: true });
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        if (!open) openList(true);
        else items()[items().length - 1]?.focus({ preventScroll: true });
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div${attributes({
        class: clsx(`share-picker ${className}`.trim()),
        ...restProps
      })}>`);
      IconButton($$renderer3, {
        baseClass: "share-picker-button",
        label,
        "aria-expanded": open,
        "aria-controls": listId,
        onclick: onButtonClick,
        onkeydown: onButtonKeydown,
        get ref() {
          return buttonEl;
        },
        set ref($$value) {
          buttonEl = $$value;
          $$settled = false;
        },
        children: ($$renderer4) => {
          if (children) {
            $$renderer4.push("<!--[0-->");
            children($$renderer4, { open, url: currentUrl() });
            $$renderer4.push(`<!---->`);
          } else {
            $$renderer4.push(`<!--[-1--><svg class="share-picker-icon" viewBox="0 0 16 16" width="1.05rem" height="1.05rem" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 8h11M9 3.5 13.5 8 9 12.5"></path></svg>`);
          }
          $$renderer4.push(`<!--]-->`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!---->  <ul class="share-picker-list"${attr("id", listId)}${attr("aria-label", label)}${attr("hidden", !open)}><!--[-->`);
      const each_array = ensure_array_like(targets);
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let target = each_array[$$index];
        $$renderer3.push(`<li class="share-picker-list-item"><a class="share-picker-target"${attr("data-target-id", target.id)}${attr("href", target.href(currentUrl(), title, text))}${attr("target", target.newTab === false ? void 0 : "_blank")} rel="noopener noreferrer">${escape_html(target.label)}</a></li>`);
      }
      $$renderer3.push(`<!--]--> `);
      if (copyLabel) {
        $$renderer3.push(`<!--[0--><li class="share-picker-list-item"><button type="button" class="share-picker-copy">${escape_html(copyLabel)}</button></li>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--></ul> <p class="share-picker-status" aria-live="polite">${escape_html(status)}</p></div>`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
  });
}
const DEFAULT_THEMES = [
  "abyss",
  "acid",
  "adobe-spectrum",
  "aqua",
  "autumn",
  "black",
  "bumblebee",
  "business",
  "caramellatte",
  "cmyk",
  "coffee",
  "corporate",
  "cupcake",
  "cyberpunk",
  "dark",
  "dim",
  "dracula",
  "emerald",
  "fantasy",
  "forest",
  "garden",
  "halloween",
  "lemonade",
  "light",
  "lofi",
  "luxury",
  "mozilla-protocol",
  "night",
  "nord",
  "pastel",
  "retro",
  "silk",
  "sunset",
  "synthwave",
  "valentine",
  "winter",
  "wireframe",
  "united-kingdom-government-digital-service",
  "united-kingdom-national-health-service-england-for-patients",
  "united-kingdom-national-health-service-england-for-practitioners",
  "united-kingdom-national-health-service-scotland-for-patients",
  "united-kingdom-national-health-service-scotland-for-practitioners",
  "united-kingdom-national-health-service-wales-for-patients",
  "united-kingdom-national-health-service-wales-for-practitioners",
  "united-states-web-design-system"
];
const DEFAULT_SIZES = [
  "largest",
  "larger",
  "large",
  "normal",
  "small",
  "smaller",
  "smallest"
];
function PickerBar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className = "",
      labels,
      themesUrl,
      themes = DEFAULT_THEMES,
      themeProps = {},
      locales,
      localeProps = {},
      sizes = DEFAULT_SIZES,
      textSizeProps = {},
      shareTargets = [],
      shareProps = {},
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    $$renderer2.push(`<div${attributes({
      class: clsx(`picker-bar ${className}`.trim()),
      ...restProps
    })}>`);
    ThemePicker($$renderer2, spread_props([{ label: labels.theme, themesUrl, themes }, themeProps]));
    $$renderer2.push(`<!----> `);
    LocalePicker($$renderer2, spread_props([{ label: labels.locale, locales }, localeProps]));
    $$renderer2.push(`<!----> `);
    TextSizePicker($$renderer2, spread_props([
      { label: labels.textSize, sizes, defaultValue: "normal" },
      textSizeProps
    ]));
    $$renderer2.push(`<!----> `);
    SharePicker($$renderer2, spread_props([{ label: labels.share, targets: shareTargets }, shareProps]));
    $$renderer2.push(`<!----></div>`);
  });
}
function _layout($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { children, data } = $$props;
    const signedIn = derived(() => data.signedIn);
    let menuOpen = false;
    const SHARE_TARGETS = derived(() => [
      {
        id: "email",
        label: t("share.email"),
        href: (url, title) => `mailto:?subject=${encodeURIComponent(title)}&body=${encodeURIComponent(url)}`,
        newTab: false
      },
      {
        id: "linkedin",
        label: t("share.linkedin"),
        href: (url) => `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`
      },
      {
        id: "reddit",
        label: t("share.reddit"),
        href: (url, title) => `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`
      },
      {
        id: "bluesky",
        label: t("share.bluesky"),
        href: (url, title) => `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`
      },
      {
        id: "mastodon",
        label: t("share.mastodon"),
        href: (url, title) => `https://mastodonshare.com/?text=${encodeURIComponent(title)}&url=${encodeURIComponent(url)}`
      }
    ]);
    const pageTitle = derived(() => page.data?.title ?? t("brand.name"));
    const THEME_KEY = "mxi.wpm.theme";
    const THEME_KEY_LEGACY = "mxi.hcm.theme";
    if (typeof localStorage !== "undefined") {
      try {
        const legacy = localStorage.getItem(THEME_KEY_LEGACY);
        if (legacy !== null && localStorage.getItem(THEME_KEY) === null) {
          localStorage.setItem(THEME_KEY, legacy);
          localStorage.removeItem(THEME_KEY_LEGACY);
        }
      } catch {
      }
    }
    const navItems = [
      { href: "/", key: "nav.dashboard" },
      { href: "/employees", key: "nav.employees" },
      { href: "/org-chart", key: "nav.orgChart" },
      { href: "/requisitions", key: "nav.requisitions" },
      { href: "/workforce", key: "nav.workforce" },
      { href: "/development", key: "nav.development" },
      { href: "/learning", key: "nav.learning" },
      { href: "/mentorship", key: "nav.mentorship" },
      { href: "/wellbeing", key: "nav.wellbeing" },
      { href: "/privacy", key: "nav.privacy" },
      { href: "/payroll", key: "nav.payroll" },
      { href: "/benchmarks", key: "nav.benchmarks" },
      { href: "/tour", key: "nav.tour" }
    ];
    $$renderer2.push(`<div class="layout svelte-12qhfyh"><header class="topbar svelte-12qhfyh"><button type="button" class="hamburger svelte-12qhfyh"${attr("aria-expanded", menuOpen)} aria-controls="primary-nav"${attr("aria-label", t("nav.toggle"))}><span class="hamburger-box svelte-12qhfyh" aria-hidden="true"></span></button> <a href="/" class="brand svelte-12qhfyh">${escape_html(t("brand.name"))}</a> <nav id="primary-nav"${attr_class("primary-nav svelte-12qhfyh", void 0, { "open": menuOpen })}><ul class="svelte-12qhfyh"><!--[-->`);
    const each_array = ensure_array_like(navItems);
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let item = each_array[$$index];
      $$renderer2.push(`<li><a${attr("href", item.href)}${attr("aria-current", page.url.pathname === item.href ? "page" : null)} class="svelte-12qhfyh">${escape_html(t(item.key))}</a></li>`);
    }
    $$renderer2.push(`<!--]--></ul></nav> <div class="header-end svelte-12qhfyh">`);
    if (signedIn()) {
      $$renderer2.push(`<!--[0--><form method="POST" action="/signout" class="svelte-12qhfyh"><button type="submit" class="session-button svelte-12qhfyh">${escape_html(t("auth.signout"))}</button></form>`);
    } else {
      $$renderer2.push(`<!--[-1--><a class="session-button signin svelte-12qhfyh" href="/signin">${escape_html(t("auth.signin"))}</a>`);
    }
    $$renderer2.push(`<!--]--> `);
    PickerBar($$renderer2, {
      labels: {
        theme: t("nav.theme"),
        locale: t("chrome.language"),
        textSize: t("nav.text_size"),
        share: t("nav.share")
      },
      themesUrl: "/assets/themes/",
      themeProps: {
        storageKey: THEME_KEY,
        detectFromSystem: true,
        defaultValue: "light"
      },
      locales: [...LOCALES],
      localeProps: {
        value: i18n.locale,
        localeLabels: LOCALE_LABELS,
        applyDir: false,
        onChange: (code) => i18n.set(code)
      },
      textSizeProps: { storageKey: "mxi.wpm.text-size" },
      shareTargets: SHARE_TARGETS(),
      shareProps: {
        title: pageTitle(),
        copyLabel: t("share.copy_link"),
        copiedLabel: t("share.copied"),
        copyFailedLabel: t("share.copy_failed")
      }
    });
    $$renderer2.push(`<!----></div></header> <main>`);
    children($$renderer2);
    $$renderer2.push(`<!----></main></div>`);
  });
}
export {
  _layout as default
};
