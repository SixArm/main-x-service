<!--
  Learning area (`/learning`): the per-department skills matrix + the
  declared-gap list, per-department training analytics (completion
  ratios carry numerator/denominator; cert-expiry counts), and
  learning-path progress (honest — a step counts only against a
  completed training enrolment). All server-derived; derivations shown.
-->
<script lang="ts">
  import {
    listPaths,
    pathProgress,
    skillsMatrix,
    trainingAnalytics,
  } from "$lib/api/wpm";
  import { t } from "$lib/i18n.svelte";

  type Matrix = Awaited<ReturnType<typeof skillsMatrix>>;
  type Analytics = Awaited<ReturnType<typeof trainingAnalytics>>;
  type Paths = Awaited<ReturnType<typeof listPaths>>;
  type Progress = Awaited<ReturnType<typeof pathProgress>>;

  let matrix = $state<Matrix | null>(null);
  let analytics = $state<Analytics | null>(null);
  let paths = $state<Paths>([]);
  let selectedPath = $state("");
  let progress = $state<Progress | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        matrix = await skillsMatrix();
        analytics = await trainingAnalytics();
        paths = await listPaths();
        if (!selectedPath && paths.length > 0) {
          selectedPath = paths[0]?.pid ?? "";
        }
        if (selectedPath) progress = await pathProgress(selectedPath);
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  async function loadProgress(pid: string) {
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

  const pct = (done: number, total: number) =>
    total === 0 ? "—" : `${Math.round((done / total) * 100)}%`;
</script>

<svelte:head><title>{t("nav.learning")} — WPM</title></svelte:head>

<h1>{t("nav.learning")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if matrix}
  <h2>Skills matrix</h2>
  <p class="muted">{matrix.note}</p>
  <table data-testid="skills-matrix">
    <thead>
      <tr><th>Department</th><th>Skill</th><th>Employees</th><th>Avg proficiency</th><th>Below target</th></tr>
    </thead>
    <tbody>
      {#each matrix.matrix as cell (cell.department + cell.skill)}
        <tr>
          <td>{cell.department}</td>
          <td>{cell.skill ?? "—"}</td>
          <td>{cell.employees}</td>
          <td>{cell.average_proficiency.toFixed(1)}</td>
          <td class:warn={cell.below_target > 0}>{cell.below_target}</td>
        </tr>
      {:else}
        <tr><td colspan="5" class="muted">No declared skills yet.</td></tr>
      {/each}
    </tbody>
  </table>
  {#if matrix.gaps.length > 0}
    <h3>Skill gaps</h3>
    <ul data-testid="skills-gaps">
      {#each matrix.gaps as gap, index (index)}
        <li>{gap.skill} in {gap.department}: {gap.proficiency} → target {gap.target}</li>
      {/each}
    </ul>
  {/if}
{/if}

{#if analytics}
  <h2>Training analytics</h2>
  <p class="muted">{analytics.note} · certs expiring by {analytics.horizon}</p>
  <table data-testid="training-analytics">
    <thead>
      <tr><th>Department</th><th>Completion</th><th>Certs expiring</th></tr>
    </thead>
    <tbody>
      {#each analytics.departments as dept (dept.department)}
        <tr>
          <td>{dept.department}</td>
          <td>
            {dept.completion_rate.value === null
              ? "—"
              : `${Math.round(dept.completion_rate.value * 100)}%`}
            ({dept.completion_rate.numerator}/{dept.completion_rate.denominator})
          </td>
          <td>{dept.certs_expiring}</td>
        </tr>
      {:else}
        <tr><td colspan="3" class="muted">No training enrolments yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<h2>Learning path progress</h2>
{#if paths.length > 0}
  <p>
    <label>
      <select
        data-testid="path-select"
        value={selectedPath}
        onchange={(event) => void loadProgress(event.currentTarget.value)}
      >
        {#each paths as path (path.pid)}
          <option value={path.pid}>{path.name} ({path.steps} steps)</option>
        {/each}
      </select>
    </label>
  </p>
{:else}
  <p class="muted">No learning paths defined.</p>
{/if}

{#if progress}
  <p class="muted">{progress.derivation}</p>
  <table data-testid="path-progress">
    <thead><tr><th>Employee</th><th>Completed</th><th>Progress</th></tr></thead>
    <tbody>
      {#each progress.members as member (member.employee_pid)}
        <tr>
          <td>{member.display_name ?? member.employee_pid}</td>
          <td>{member.completed_steps} / {member.total_steps}</td>
          <td>{pct(member.completed_steps, member.total_steps)}</td>
        </tr>
      {:else}
        <tr><td colspan="3" class="muted">No one enrolled.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  td.warn { color: #b45309; font-weight: 600; }
</style>
