import { j as head, g as ensure_array_like } from "../../../chunks/index2.js";
import { p as percentOf } from "../../../chunks/format.js";
import { t } from "../../../chunks/i18n.svelte.js";
import { e as escape_html } from "../../../chunks/escaping.js";
const API_BASE_URL = "/api/proxy";
class ApiError extends Error {
  status;
  body;
  constructor(status, body) {
    const description = typeof body === "object" && body !== null && "description" in body ? String(body.description) : `API error ${status}`;
    super(description);
    this.status = status;
    this.body = body;
  }
}
async function api(path, init) {
  const doFetch = fetch;
  const url = `${API_BASE_URL}${path}`;
  const response = await doFetch(url, {
    method: "GET",
    headers: {},
    body: void 0
  });
  const text = await response.text();
  const parsed = text ? JSON.parse(text) : null;
  if (!response.ok) throw new ApiError(response.status, parsed);
  return parsed;
}
function pathProgress(pathPid, init) {
  return api(`/learning-paths/${pathPid}/progress`);
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let paths = [];
    let selectedPath = "";
    let progress = null;
    let error = null;
    async function loadProgress(pid) {
      selectedPath = pid;
      progress = null;
      if (pid) {
        try {
          progress = await pathProgress(pid);
        } catch (cause) {
          error = cause instanceof Error ? cause.message : String(cause);
        }
      }
    }
    head("1lta5mn", $$renderer2, ($$renderer3) => {
      $$renderer3.title(($$renderer4) => {
        $$renderer4.push(`<title>${escape_html(t("nav.learning"))} — WPM</title>`);
      });
    });
    $$renderer2.push(`<h1>${escape_html(t("nav.learning"))}</h1> `);
    if (error) {
      $$renderer2.push(`<!--[0--><p class="error" data-testid="error">${escape_html(error)}</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <h2>Learning path progress</h2> `);
    if (paths.length > 0) {
      $$renderer2.push(`<!--[0--><p><label>`);
      $$renderer2.select(
        {
          "data-testid": "path-select",
          value: selectedPath,
          onchange: (event) => void loadProgress(event.currentTarget.value)
        },
        ($$renderer3) => {
          $$renderer3.push(`<!--[-->`);
          const each_array_3 = ensure_array_like(paths);
          for (let $$index_3 = 0, $$length = each_array_3.length; $$index_3 < $$length; $$index_3++) {
            let path = each_array_3[$$index_3];
            $$renderer3.option({ value: path.pid }, ($$renderer4) => {
              $$renderer4.push(`${escape_html(path.name)} (${escape_html(path.steps)} steps)`);
            });
          }
          $$renderer3.push(`<!--]-->`);
        }
      );
      $$renderer2.push(`</label></p>`);
    } else {
      $$renderer2.push(`<!--[-1--><p class="muted">No learning paths defined.</p>`);
    }
    $$renderer2.push(`<!--]--> `);
    if (progress) {
      $$renderer2.push(`<!--[0--><p class="muted">${escape_html(progress.derivation)}</p> <table data-testid="path-progress"><thead><tr><th>Employee</th><th>Completed</th><th>Progress</th></tr></thead><tbody>`);
      const each_array_4 = ensure_array_like(progress.members);
      if (each_array_4.length !== 0) {
        $$renderer2.push("<!--[-->");
        for (let $$index_4 = 0, $$length = each_array_4.length; $$index_4 < $$length; $$index_4++) {
          let member = each_array_4[$$index_4];
          $$renderer2.push(`<tr><td>${escape_html(member.display_name ?? member.employee_pid)}</td><td>${escape_html(member.completed_steps)} / ${escape_html(member.total_steps)}</td><td>${escape_html(percentOf(member.completed_steps, member.total_steps))}</td></tr>`);
        }
      } else {
        $$renderer2.push(`<!--[!--><tr><td colspan="3" class="muted">No one enrolled.</td></tr>`);
      }
      $$renderer2.push(`<!--]--></tbody></table>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
