<!--
  Tour (/tour) — a public, in-depth walkthrough of the plan registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order. Per-plan screens
  (board, flow, governance) are reached from a plan's detail page, so those
  sections open the plan list.
-->
<script lang="ts">
  import Tour from "$lib/components/Tour.svelte";

  const sections = [
    { href: "/plans/new" },
    { href: "/plans" },
    { href: "/plans" },
    { href: "/plans" },
    { href: "/ideas" },
    { href: "/plans" },
  ];
</script>

<Tour {sections} />
