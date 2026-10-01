<!--
  Tour (/tour) — a public, in-depth walkthrough of the sign-in service.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "#lib/components/Tour.svelte";

    const sections = [
        { href: "/signup" },
        { href: "/signin" },
        { href: "/signin" },
        { href: "/" },
        { href: "/" },
        { href: "/admin/attributes" },
    ];
</script>

<Tour {sections} />
