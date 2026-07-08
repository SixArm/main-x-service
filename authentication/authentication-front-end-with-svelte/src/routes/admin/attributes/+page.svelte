<!--
  Operator UI: view / edit a user's ABAC subject attributes.

  Pick a user by pasting their pid (GET form → `?pid=…`); the server
  loads their current attributes. Edit the JSON map and Save to PUT it.
  Requires the signed-in operator to carry `access=admin` — otherwise the
  service replies 403 and we show it. Operator-facing internal tool, so
  the copy is plain English (not localised).
-->
<script lang="ts">
    import type { PageData, ActionData } from "./$types";
    import { enhance } from "$app/forms";
    import { t } from "$lib/i18n.svelte";

    let { data, form }: { data: PageData; form: ActionData } = $props();

    // Editor contents: a just-saved result wins, else the loaded target,
    // else an empty object — pretty-printed for editing.
    let editorText = $derived(
        JSON.stringify(form?.attributes ?? data.target?.attributes ?? {}, null, 2),
    );
</script>

<svelte:head><title>Attributes — {t("brand")}</title></svelte:head>

<h1>User attributes (ABAC)</h1>
<p>
    <small>
        Assign a user's <code>access</code> / <code>dept</code> /
        <code>svc</code> attributes. Requires an <code>access=admin</code> session.
    </small>
</p>

<!-- Choose the target user by pid (GET → ?pid=…). -->
<form class="stack" method="GET">
    <label>
        User id (pid)
        <input
            type="text"
            name="pid"
            value={data.pid ?? ""}
            placeholder="00000000-0000-0000-0000-000000000000"
            autocomplete="off"
            required
        />
    </label>
    <button class="button" type="submit">Load</button>
</form>

{#if data.error}
    <p class="banner" role="alert">
        {data.status ? `${data.status}: ` : ""}{data.error}
    </p>
{/if}

{#if data.target}
    <h2>{data.target.email}</h2>
    <p><small><code>{data.target.pid}</code></small></p>

    {#if form?.saved}
        <p class="banner">Attributes saved.</p>
    {:else if form?.message}
        <p class="banner" role="alert">{form.message}</p>
    {/if}

    <form class="stack" method="POST" action="?/save" use:enhance>
        <input type="hidden" name="pid" value={data.target.pid} />
        <label>
            Attributes (JSON: <code>{"{ \"access\": [\"write\"] }"}</code>)
            <textarea name="attributes" rows="10" spellcheck="false"
                >{editorText}</textarea
            >
        </label>
        <button class="button" type="submit">Save</button>
        <p>
            <small>
                Send <code>{"{}"}</code> to clear every attribute (user becomes
                read-only under the default policy).
            </small>
        </p>
    </form>
{/if}
