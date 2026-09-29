<!--
  Tour (/tour) — a public, in-depth walkthrough of the Workforce Planning
  Management app. Copy lives in the i18n catalog (`tour.*`); this file only
  declares which app screen each workflow section links to, in order.
-->
<script lang="ts">
  import Tour from "$lib/components/Tour.svelte";

  const sections = [
    { href: "/employees" },
    { href: "/requisitions" },
    { href: "/workforce" },
    { href: "/payroll" },
    { href: "/learning" },
    { href: "/wellbeing" },
  ];
</script>

<Tour {sections} />
