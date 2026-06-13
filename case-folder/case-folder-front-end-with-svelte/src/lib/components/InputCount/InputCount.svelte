<script lang="ts">
    // InputCount component
    //
    // A small numeric stepper: − [number] +. Headless.
    //
    // Props:
    //   value — number, bindable. Current count.
    //   min   — number, default 1.
    //   max   — number, default 99.
    //   label — string, required. Accessible label for the number field.
    //   class — string, optional.

    import Icon from '$lib/components/Icon/Icon.svelte';

    let {
        value = $bindable(1),
        min = 1,
        max = 99,
        label,
        class: className = ''
    }: {
        value?: number;
        min?: number;
        max?: number;
        label: string;
        class?: string;
    } = $props();

    function clamp(n: number): number {
        if (Number.isNaN(n)) return min;
        return Math.min(max, Math.max(min, Math.round(n)));
    }

    const decrement = () => (value = clamp(value - 1));
    const increment = () => (value = clamp(value + 1));
</script>

<div class={`input-count ${className}`}>
    <button
        type="button"
        class="input-count-step"
        aria-label="Decrease {label}"
        disabled={value <= min}
        onclick={decrement}
    >
        <Icon name="minus" />
    </button>
    <input
        class="input-count-value"
        type="number"
        {min}
        {max}
        aria-label={label}
        bind:value
        onchange={() => (value = clamp(value))}
    />
    <button
        type="button"
        class="input-count-step"
        aria-label="Increase {label}"
        disabled={value >= max}
        onclick={increment}
    >
        <Icon name="plus" />
    </button>
</div>
