<!--
  The block editor.

  It edits the **structured block model** and posts blocks
  (`../../spec/authoring.md`). There is no `contenteditable`, nothing
  serializes to markup, and no block body is ever rendered with
  `{@html}` — the service sanitizes on write as a boundary control,
  which is not permission to trust its output blindly here.

  Per-block problems are shown as the editor types, but they are
  advisory: the service is the authority, and these exist to save a
  round trip rather than to replace its answer.
-->
<script lang="ts">
  import { BLOCK_KINDS, blank, insertAt, move, problems, removeAt, replaceAt } from "$lib/blocks";
  import type { BlockKind } from "$lib/blocks";
  import type { Block } from "$lib/api/cms";
  import { t } from "$lib/i18n.svelte";

  let { blocks = $bindable() }: { blocks: Block[] } = $props();

  let adding = $state<BlockKind>("paragraph");

  function update(index: number, patch: Record<string, unknown>) {
    const current = blocks[index];
    if (!current) return;
    blocks = replaceAt(blocks, index, { ...current, ...patch });
  }

  function items(block: Block): string[] {
    return Array.isArray(block.items) ? block.items.map(String) : [];
  }
</script>

<section class="panel">
  <h2>{t("entry.blocks")}</h2>

  {#each blocks as block, index (index)}
    <article class="block">
      <header>
        <strong>{block.kind}</strong>
        <span class="spacer"></span>
        <button type="button" onclick={() => (blocks = move(blocks, index, index - 1))}
          disabled={index === 0}>↑<span class="visually-hidden">{t("common.moveUp")}</span></button>
        <button type="button" onclick={() => (blocks = move(blocks, index, index + 1))}
          disabled={index === blocks.length - 1}>↓<span class="visually-hidden">{t("common.moveDown")}</span></button>
        <button type="button" onclick={() => (blocks = removeAt(blocks, index))}>
          {t("common.remove")}
        </button>
      </header>

      {#if block.kind === "heading"}
        <label>
          {t("common.title")}
          <input value={String(block.text ?? "")}
            oninput={(e) => update(index, { text: e.currentTarget.value })} />
        </label>
        <label>
          level
          <input type="number" min="2" max="6" value={Number(block.level ?? 2)}
            oninput={(e) => update(index, { level: Number(e.currentTarget.value) })} />
        </label>
      {:else if block.kind === "list"}
        {#each items(block) as item, at (at)}
          <input value={item}
            oninput={(e) => {
              const next = items(block);
              next[at] = e.currentTarget.value;
              update(index, { items: next });
            }} />
        {/each}
        <button type="button" onclick={() => update(index, { items: [...items(block), ""] })}>
          +
        </button>
      {:else if block.kind === "image"}
        <label>
          asset
          <input value={String(block.asset_pid ?? "")}
            oninput={(e) => update(index, { asset_pid: e.currentTarget.value })} />
        </label>
        <label>
          alt
          <input value={String(block.alt ?? "")}
            oninput={(e) => update(index, { alt: e.currentTarget.value })} />
        </label>
      {:else}
        <textarea rows="3" value={String(block.text ?? "")}
          oninput={(e) => update(index, { text: e.currentTarget.value })}></textarea>
      {/if}

      {#each problems(block) as problem (problem)}
        <p class="ahead">{problem}</p>
      {/each}
    </article>
  {/each}

  <div class="row">
    <!-- Named for the same reason every control here is: a select with
         no accessible name is unreachable by screen reader, and this is
         a tool for making accessible sites. -->
    <label class="visually-hidden" for="block-kind">{t("entry.addBlock")}</label>
    <select id="block-kind" bind:value={adding}>
      {#each BLOCK_KINDS as kind (kind)}
        <option value={kind}>{kind}</option>
      {/each}
    </select>
    <button type="button" onclick={() => (blocks = insertAt(blocks, blocks.length, blank(adding)))}>
      {t("entry.addBlock")}
    </button>
  </div>
</section>

<style>
  .block {
    border: 1px solid var(--line);
    border-radius: 0.35rem;
    padding: 0.5rem 0.6rem;
    margin-bottom: 0.5rem;
  }
  .block header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    margin-bottom: 0.4rem;
  }
  .spacer {
    flex: 1;
  }
  .block input,
  .block textarea {
    width: 100%;
    font: inherit;
  }
  .row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
