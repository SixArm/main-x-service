<script lang="ts">
  import { articleStatus, listArticles } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";
  import type { Article } from "$lib/api/crm";

  let articles = $state<Article[] | null>(null);
  let query = $state("");
  let error = $state<string | null>(null);

  async function load() {
    try {
      articles = await listArticles(query || undefined);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  async function publish(article: Article) {
    await articleStatus(article.pid, "published");
    await load();
  }
</script>

<h1>{t("nav.articles")}</h1>

<div class="panel">
  <label>
    {t("article.search")}
    <input bind:value={query} onchange={() => void load()} />
  </label>
</div>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if articles === null}
  <p>{t("common.loading")}</p>
{:else}
  <table data-testid="articles">
    <thead>
      <tr>
        <th>{t("common.name")}</th>
        <th>{t("common.status")}</th>
        <th>{t("article.version")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each articles as article (article.pid)}
        <tr>
          <td>{article.title}</td>
          <td><span class="chip">{article.status}</span></td>
          <td>v{article.version}</td>
          <td>
            {#if article.status === "draft"}
              <button onclick={() => void publish(article)}>{t("article.publish")}</button>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
