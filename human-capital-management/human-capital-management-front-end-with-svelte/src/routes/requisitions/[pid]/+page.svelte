<script lang="ts">
  import { page } from "$app/state";
  import { applicationStage, getRequisition, listApplications } from "$lib/api/hcm";
  import { t } from "$lib/i18n.svelte";
  import type { Application, Requisition } from "$lib/api/types";

  /** The forward move an application offers per stage. */
  const NEXT: Record<string, string> = {
    received: "screened",
    screened: "interviewing",
    interviewing: "offer",
  };

  let requisition = $state<Requisition | null>(null);
  let applications = $state<Application[]>([]);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let hireNumber = $state("");

  const pid = $derived(page.params.pid ?? "");

  async function load() {
    try {
      requisition = await getRequisition(pid);
      applications = await listApplications(pid);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    if (pid) void load();
  });

  async function move(application: Application, to: string) {
    actionError = null;
    try {
      await applicationStage(application.pid, {
        to,
        ...(to === "hired" ? { employee_number: hireNumber } : {}),
      });
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }
</script>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if requisition === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1>{requisition.job_title} <span class="chip">{requisition.status}</span></h1>
  <p class="muted">{requisition.department} · {t("req.headcount")} {requisition.headcount}</p>

  <h2>{t("req.applications")}</h2>
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <table data-testid="applications">
    <thead>
      <tr><th>{t("common.name")}</th><th>{t("common.status")}</th><th>{t("common.actions")}</th></tr>
    </thead>
    <tbody>
      {#each applications as application (application.pid)}
        <tr>
          <td>{application.candidate_pid.slice(0, 8)}</td>
          <td><span class="chip">{application.stage}</span></td>
          <td>
            {#if NEXT[application.stage]}
              {@const next = NEXT[application.stage] ?? ""}
              <button onclick={() => void move(application, next)}>
                → {next}
              </button>
            {/if}
            {#if application.stage === "offer"}
              <input placeholder="E-0000" bind:value={hireNumber} size="8" />
              <button onclick={() => void move(application, "hired")}>→ hired</button>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
