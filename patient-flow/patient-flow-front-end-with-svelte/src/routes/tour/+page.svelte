<!--
  Tour (/tour) — a public, in-depth walkthrough of Patient Flow.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
  import Tour from "$lib/components/Tour.svelte";

  const sections = [
    { href: "/wards" },
    { href: "/at-a-glance" },
    { href: "/bed-requests" },
    { href: "/edd" },
    { href: "/wards" },
    { href: "/audits" },
  ];
</script>

<Tour {sections} />
