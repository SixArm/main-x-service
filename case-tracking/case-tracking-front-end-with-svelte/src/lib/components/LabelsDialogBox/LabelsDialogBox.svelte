<script lang="ts" module>
    export interface LabelOption {
        id: string;
        title: string;
    }
</script>

<script lang="ts">
    // LabelsDialogBox component (spec/labels-dialog-box.md)
    //
    // A dialog for printing labels for a selection of volumes:
    //   - Title "Labels"
    //   - search text + "Find" / "Clear"
    //   - a scrollable multi-select list of volume titles
    //   - footer: "Number of Copies:" stepper, "Print", "Close"
    //
    // Headless: structure + ARIA only.
    //
    // Props:
    //   open      — boolean, bindable. Whether the dialog is shown.
    //   volumes   — LabelOption[]. The selectable volume titles.
    //   copies    — number, bindable. Number of copies to print.
    //   selected  — string[], bindable. Selected volume ids.
    //   onprint   — ({ selected, copies }) => void.
    //   onclose   — () => void.

    import TextInput from '$lib/components/TextInput/TextInput.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import InputCount from '$lib/components/InputCount/InputCount.svelte';

    let {
        open = $bindable(true),
        volumes = [],
        copies = $bindable(1),
        selected = $bindable([]),
        onprint = undefined,
        onclose = undefined,
        class: className = ''
    }: {
        open?: boolean;
        volumes?: LabelOption[];
        copies?: number;
        selected?: string[];
        onprint?: (detail: { selected: string[]; copies: number }) => void;
        onclose?: () => void;
        class?: string;
    } = $props();

    let search = $state('');
    let query = $state('');

    const filtered = $derived(
        query.trim()
            ? volumes.filter((v) => v.title.toLowerCase().includes(query.trim().toLowerCase()))
            : volumes
    );

    function find() {
        query = search;
    }
    function clear() {
        search = '';
        query = '';
        selected = [];
    }
    function print() {
        onprint?.({ selected, copies });
    }
    function close() {
        open = false;
        onclose?.();
    }
</script>

{#if open}
    <div class="dialog-backdrop">
        <div
            class={`labels-dialog ${className}`}
            role="dialog"
            aria-modal="true"
            aria-labelledby="labels-dialog-title"
        >
            <h2 id="labels-dialog-title">Labels</h2>

            <div class="labels-dialog-search">
                <TextInput label="Search labels" placeholder="Enter text to search..." bind:value={search} />
                <Button type="button" onclick={find}>Find</Button>
                <Button type="button" onclick={clear}>Clear</Button>
            </div>

            <ul class="labels-dialog-list" role="listbox" aria-multiselectable="true" aria-label="Volume titles">
                {#each filtered as option (option.id)}
                    <li role="option" aria-selected={selected.includes(option.id)}>
                        <label>
                            <input type="checkbox" value={option.id} bind:group={selected} />
                            {option.title}
                        </label>
                    </li>
                {/each}
                {#if filtered.length === 0}
                    <li class="labels-dialog-empty">No matching volumes.</li>
                {/if}
            </ul>

            <footer class="labels-dialog-footer">
                <label class="labels-dialog-copies">
                    Number of Copies:
                    <InputCount label="Number of copies" bind:value={copies} />
                </label>
                <div class="labels-dialog-actions">
                    <Button type="button" onclick={print}>Print</Button>
                    <Button type="button" onclick={close}>Close</Button>
                </div>
            </footer>
        </div>
    </div>
{/if}
