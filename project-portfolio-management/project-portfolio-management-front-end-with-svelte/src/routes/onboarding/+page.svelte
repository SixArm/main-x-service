<!--
  Operator onboarding guide (`/onboarding`, T-28p): a role-by-role
  "first hour" walkthrough. Pure static content — no API call, so
  nothing here can fail to load.

  Deliberately makes no time-to-productivity claim: nobody has
  measured how long a first hour with this guide actually takes.
-->
<script lang="ts">
  import { ONBOARDING_ROLES } from "$lib/onboarding";
</script>

<svelte:head><title>Onboarding — PPM</title></svelte:head>

<h1>Operator onboarding</h1>
<p class="small muted">
  A role-by-role first-hour walkthrough. Each step names the page it
  lands on. This is a suggested order, not a measured "you'll be
  productive in N minutes" claim — nobody has timed it.
</p>

{#each ONBOARDING_ROLES as role (role.id)}
  <section class="role">
    <h2>{role.title}</h2>
    <p class="small muted">{role.summary}</p>
    <ol>
      {#each role.steps as step, i (step.route + i)}
        <li>
          <a href={step.route}>{step.label}</a>
          <p class="small muted">{step.note}</p>
        </li>
      {/each}
    </ol>
  </section>
{/each}

<style>
  .role {
    margin-block: 1.5rem;
  }
  ol {
    padding-inline-start: 1.5rem;
  }
  li {
    margin-block-end: 0.75rem;
  }
</style>
