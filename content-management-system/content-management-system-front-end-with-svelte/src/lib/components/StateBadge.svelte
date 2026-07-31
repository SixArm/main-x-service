<!--
  An editorial state, named in words as well as colour.

  Colour alone fails a colour-blind editor and fails in a printout, and
  this is the one distinction the UI cannot afford to lose: whether
  readers can see this page.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import type { MessageKey } from "$lib/i18n.svelte";

  let { status }: { status: string } = $props();

  const LABELS: Record<string, MessageKey> = {
    draft: "entry.draft",
    in_review: "entry.inReview",
    approved: "entry.approved",
    published: "entry.published",
    archived: "entry.archived",
  };

  // An unrecognised status shows its raw token rather than being
  // silently relabelled: if the service grows a state the UI does not
  // know, saying so is safer than guessing.
  const label = $derived(LABELS[status] ? t(LABELS[status]) : status);
</script>

<span class="state {status}">{label}</span>
