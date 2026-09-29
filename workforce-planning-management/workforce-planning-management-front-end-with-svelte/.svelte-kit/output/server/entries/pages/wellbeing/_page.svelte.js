import { g as ensure_array_like, f as attr, k as stringify } from "../../../chunks/index2.js";
import { m as mean } from "../../../chunks/format.js";
import { t } from "../../../chunks/i18n.svelte.js";
import { e as escape_html } from "../../../chunks/escaping.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let rules = [];
    let pulse = [];
    let name = "";
    let kind = "health";
    let description = "";
    let infoUrl = "";
    let minAge = "";
    let maxAge = "";
    let departments = "";
    let doses = "1";
    function kindLabel(value) {
      return value === "benefit" ? t("wb.kind.benefit") : t("wb.kind.health");
    }
    function cell(value) {
      if (value.suppressed) return t("wb.pulseSuppressed");
      return `${t("wb.pulseMean")} ${mean(value.mean) ?? "—"} · ${value.count} ${t("wb.pulseResponses")}`;
    }
    function cohort(rule) {
      const parts = [];
      if (rule.min_age !== null || rule.max_age !== null) parts.push(`${rule.min_age ?? "0"}–${rule.max_age ?? "∞"}`);
      if (rule.departments.length) parts.push(rule.departments.join(", "));
      if (rule.job_titles.length) parts.push(rule.job_titles.join(", "));
      return parts.length ? parts.join(" · ") : "—";
    }
    $$renderer2.push(`<h1>${escape_html(t("nav.wellbeing"))}</h1> `);
    {
      $$renderer2.push(`<!--[-1--><h2>${escape_html(t("wb.rules"))}</h2> <table data-testid="wellbeing-rules"><tbody><!--[-->`);
      const each_array = ensure_array_like(rules);
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let rule = each_array[$$index];
        $$renderer2.push(`<tr><td><strong>${escape_html(rule.name)}</strong> <span class="chip">${escape_html(kindLabel(rule.kind))}</span><br/><span class="muted">${escape_html(rule.description)}</span></td><td>${escape_html(cohort(rule))}</td><td>×${escape_html(rule.doses)}</td><td><button>✕</button></td></tr>`);
      }
      $$renderer2.push(`<!--]--></tbody></table> <form class="panel" data-testid="wellbeing-form"><input${attr("placeholder", t("common.name"))}${attr("value", name)} required=""/> `);
      $$renderer2.select({ value: kind }, ($$renderer3) => {
        $$renderer3.option({ value: "health" }, ($$renderer4) => {
          $$renderer4.push(`${escape_html(t("wb.kind.health"))}`);
        });
        $$renderer3.option({ value: "benefit" }, ($$renderer4) => {
          $$renderer4.push(`${escape_html(t("wb.kind.benefit"))}`);
        });
      });
      $$renderer2.push(` <input placeholder="Description"${attr("value", description)} required=""/> <input placeholder="Info URL"${attr("value", infoUrl)}/> <input placeholder="Min age"${attr("value", minAge)} inputmode="numeric"/> <input placeholder="Max age"${attr("value", maxAge)} inputmode="numeric"/> <input${attr("placeholder", `${stringify(t("common.department"))} (a, b)`)}${attr("value", departments)}/> <input placeholder="Doses"${attr("value", doses)} inputmode="numeric"/> <button type="submit">+</button> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--></form> `);
      if (pulse.length) {
        $$renderer2.push(`<!--[0--><h2>${escape_html(t("wb.pulse"))}</h2> <!--[-->`);
        const each_array_1 = ensure_array_like(pulse);
        for (let $$index_2 = 0, $$length = each_array_1.length; $$index_2 < $$length; $$index_2++) {
          let result = each_array_1[$$index_2];
          $$renderer2.push(`<div class="panel" data-testid="pulse-results"><strong>${escape_html(result.survey.name)}</strong> — ${escape_html(result.survey.question)}<br/> <span data-testid="pulse-overall">${escape_html(cell(result.overall))}</span> <table><tbody><!--[-->`);
          const each_array_2 = ensure_array_like(result.departments);
          for (let $$index_1 = 0, $$length2 = each_array_2.length; $$index_1 < $$length2; $$index_1++) {
            let department = each_array_2[$$index_1];
            $$renderer2.push(`<tr><td>${escape_html(department.department)}</td><td>${escape_html(cell(department))}</td></tr>`);
          }
          $$renderer2.push(`<!--]--></tbody></table> <p class="muted">${escape_html(result.derivation)}</p></div>`);
        }
        $$renderer2.push(`<!--]-->`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]-->`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
