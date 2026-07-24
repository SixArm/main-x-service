<script lang="ts">
  import { expiringTraining, listSuccession, successionGaps } from "$lib/api/wpm";
  import { t } from "$lib/i18n.svelte";
  import type { SuccessionEntry, TrainingEnrollment } from "$lib/api/types";

  let succession = $state<SuccessionEntry[] | null>(null);
  let gaps = $state<SuccessionEntry["plan"][]>([]);
  let expiring = $state<TrainingEnrollment[]>([]);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const [plans, gapReport, training] = await Promise.all([
          listSuccession(),
          successionGaps(),
          expiringTraining(90),
        ]);
        succession = plans;
        gaps = gapReport.gaps;
        expiring = training.expiring;
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<h1>{t("nav.development")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if succession === null}
  <p>{t("common.loading")}</p>
{:else}
  <h2>{t("dash.successionGaps")}</h2>
  <ul class="panel" data-testid="gaps">
    {#each gaps as gap (gap.pid)}
      <li><strong>{gap.role_title}</strong> — {gap.department} (criticality {gap.criticality})</li>
    {:else}
      <li class="muted">—</li>
    {/each}
  </ul>

  <h2>{t("dev.succession")}</h2>
  <table data-testid="succession">
    <tbody>
      {#each succession as entry (entry.plan.pid)}
        <tr>
          <td>{entry.plan.role_title}</td>
          <td>{entry.plan.department}</td>
          <td>{entry.plan.criticality}</td>
          <td>
            {#each entry.candidates as candidate (candidate.pid)}
              <span class="chip">#{candidate.rank} {candidate.readiness}</span>
            {/each}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("dev.expiring")}</h2>
  <table data-testid="expiring">
    <tbody>
      {#each expiring as enrollment (enrollment.pid)}
        <tr>
          <td>{enrollment.course_ref}</td>
          <td>{enrollment.certificate_expires_on}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
