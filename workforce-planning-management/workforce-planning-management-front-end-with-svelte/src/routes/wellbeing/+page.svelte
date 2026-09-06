<script lang="ts">
  // HR wellbeing admin: the configured health-entitlement rules
  // (non-clinical predicates only — age band, departments, job titles;
  // WPM-D17) and the aggregate-only uptake view (counts, never an
  // individual).
  import {
    createWellbeingEntitlement,
    deleteWellbeingEntitlement,
    listPulseSurveys,
    listWellbeingEntitlements,
    pulseResults,
    wellbeingUptake,
    type WellbeingEntitlement,
  } from "$lib/api/wpm";
  import { mean, percentWithWorkings } from "$lib/format";
  import { t } from "$lib/i18n.svelte";

  type Uptake = Awaited<ReturnType<typeof wellbeingUptake>>;
  type PulseResult = Awaited<ReturnType<typeof pulseResults>>;

  let rules = $state<WellbeingEntitlement[]>([]);
  let uptake = $state<Uptake | null>(null);
  let pulse = $state<PulseResult[]>([]);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  // Create form (kept as strings; parsed on submit).
  let name = $state("");
  let kind = $state<"health" | "benefit">("health");
  let description = $state("");
  let infoUrl = $state("");
  let minAge = $state("");
  let maxAge = $state("");
  let departments = $state("");
  let doses = $state("1");

  async function load() {
    try {
      [rules, uptake] = await Promise.all([listWellbeingEntitlements(), wellbeingUptake()]);
      const surveys = await listPulseSurveys();
      pulse = await Promise.all(surveys.map((survey) => pulseResults(survey.pid)));
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  function csv(raw: string): string[] {
    return raw
      .split(",")
      .map((entry) => entry.trim())
      .filter(Boolean);
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    actionError = null;
    try {
      await createWellbeingEntitlement({
        name,
        kind,
        description,
        info_url: infoUrl.trim() || null,
        min_age: minAge.trim() ? Number(minAge) : null,
        max_age: maxAge.trim() ? Number(maxAge) : null,
        departments: csv(departments),
        doses: Number(doses) || 1,
      });
      name = description = infoUrl = minAge = maxAge = departments = "";
      doses = "1";
      kind = "health";
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function close(pid: string) {
    actionError = null;
    try {
      await deleteWellbeingEntitlement(pid);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function kindLabel(value: "health" | "benefit"): string {
    return value === "benefit" ? t("wb.kind.benefit") : t("wb.kind.health");
  }

  function cell(value: PulseResult["overall"]): string {
    if (value.suppressed) return t("wb.pulseSuppressed");
    // `mean` guards the `undefined` case (should not happen once
    // disclosed, but a silent "undefined" in the label would be a worse
    // failure than a "—" placeholder) — see $lib/format.ts (WPM-T39).
    return `${t("wb.pulseMean")} ${mean(value.mean) ?? "—"} · ${value.count} ${t("wb.pulseResponses")}`;
  }

  function cohort(rule: WellbeingEntitlement): string {
    const parts: string[] = [];
    if (rule.min_age !== null || rule.max_age !== null)
      parts.push(`${rule.min_age ?? "0"}–${rule.max_age ?? "∞"}`);
    if (rule.departments.length) parts.push(rule.departments.join(", "));
    if (rule.job_titles.length) parts.push(rule.job_titles.join(", "));
    return parts.length ? parts.join(" · ") : "—";
  }
</script>

<h1>{t("nav.wellbeing")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else}
  <h2>{t("wb.rules")}</h2>
  <table data-testid="wellbeing-rules">
    <tbody>
      {#each rules as rule (rule.pid)}
        <tr>
          <td><strong>{rule.name}</strong> <span class="chip">{kindLabel(rule.kind)}</span><br /><span class="muted">{rule.description}</span></td>
          <td>{cohort(rule)}</td>
          <td>×{rule.doses}</td>
          <td><button onclick={() => void close(rule.pid)}>✕</button></td>
        </tr>
      {/each}
    </tbody>
  </table>

  <form class="panel" onsubmit={(event) => void create(event)} data-testid="wellbeing-form">
    <input placeholder={t("common.name")} bind:value={name} required />
    <select bind:value={kind}>
      <option value="health">{t("wb.kind.health")}</option>
      <option value="benefit">{t("wb.kind.benefit")}</option>
    </select>
    <input placeholder="Description" bind:value={description} required />
    <input placeholder="Info URL" bind:value={infoUrl} />
    <input placeholder="Min age" bind:value={minAge} inputmode="numeric" />
    <input placeholder="Max age" bind:value={maxAge} inputmode="numeric" />
    <input placeholder="{t('common.department')} (a, b)" bind:value={departments} />
    <input placeholder="Doses" bind:value={doses} inputmode="numeric" />
    <button type="submit">+</button>
    {#if actionError}
      <p class="error" data-testid="action-error">{actionError}</p>
    {/if}
  </form>

  {#if pulse.length}
    <h2>{t("wb.pulse")}</h2>
    {#each pulse as result (result.survey.pid)}
      <div class="panel" data-testid="pulse-results">
        <strong>{result.survey.name}</strong> — {result.survey.question}<br />
        <span data-testid="pulse-overall">{cell(result.overall)}</span>
        <table>
          <tbody>
            {#each result.departments as department (department.department)}
              <tr>
                <td>{department.department}</td>
                <td>{cell(department)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="muted">{result.derivation}</p>
      </div>
    {/each}
  {/if}

  {#if uptake}
    <h2>{t("wb.uptake")}</h2>
    <p class="muted">{uptake.derivation}</p>
    <table data-testid="wellbeing-uptake">
      <tbody>
        {#each uptake.entitlements as row (row.entitlement_pid)}
          <tr>
            <td>{row.name}</td>
            <td>
              {#each Object.entries(row.by_response) as [response, count] (response)}
                <span class="chip">{response}: {count}</span>
              {/each}
            </td>
            <td>{percentWithWorkings(row.uptake_rate)}</td>
            <td>
              {#if row.enrolment_conversion}
                {t("wb.conversion")}: {percentWithWorkings(row.enrolment_conversion)}
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/if}
