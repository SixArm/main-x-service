# TextSizePicker (Svelte helper)

A reusable Svelte 5 headless **text-size picker** — an icon button that
opens a WAI-ARIA APG listbox of text-size slugs. On every change it sets
`data-text-size="{slug}"` on a target element (default
`document.documentElement`), optionally persisting the choice to
`localStorage`. Ships no CSS — the consumer maps each size slug to
real typography via CSS, e.g.:

```css
:root[data-text-size="small"] {
  font-size: 87.5%;
}
:root[data-text-size="medium"] {
  font-size: 100%;
}
:root[data-text-size="large"] {
  font-size: 112.5%;
}
:root[data-text-size="x-large"] {
  font-size: 125%;
}
```

This supports WCAG 2.2 — 1.4.4 (Resize Text) and 1.4.12 (Text
Spacing) — by letting users pick a comfortable reading size that the
app remembers.

## Usage

```svelte
<script lang="ts">
  import TextSizePicker from "@lilydesignsystem/svelte-text-size-picker";
  let size = $state("");
</script>

<TextSizePicker
  label="Text size"
  sizes={["small", "medium", "large", "x-large"]}
  bind:value={size}
  storageKey="lily-text-size"
/>
```

## Props

| Prop           | Type                     | Required | Description                                            |
| -------------- | ------------------------ | -------- | ------------------------------------------------------ |
| `label`        | `string`                 | yes      | Accessible name (`aria-label`) for the `<select>`.     |
| `sizes`        | `string[]`               | yes      | Available size slugs.                                  |
| `value`        | `string`                 | no       | Selected slug. Two-way bindable.                       |
| `defaultValue` | `string`                 | no       | Initial slug when nothing else is supplied.            |
| `storageKey`   | `string`                 | no       | If set, persist the slug to `localStorage`.            |
| `name`         | `string`                 | no       | `name` of the `<select>` (default `"text-size"`).      |
| `target`       | `HTMLElement \| null`    | no       | Element to receive `data-text-size`. Default `<html>`. |
| `sizeLabels`   | `Record<string,string>`  | no       | Pretty labels per slug.                                |
| `onChange`     | `(size: string) => void` | no       | Called after a new size is applied.                    |
| `class`        | `string`                 | no       | Extra CSS class on the root `<select>`.                |

## Behaviour

Initial value resolves from `value` > storage > `defaultValue` >
`"medium"` (if present) > `sizes[0]`. All DOM writes happen inside
`$effect`, so the component is SSR-safe.

## Accessibility

- WCAG 2.2 AAA target.
- APG listbox keyboard contract: arrows (clamped), `Home` / `End`,
  `PageUp` / `PageDown` (by ten), typeahead with same-character
  cycling, `Escape` discards, and `Tab` closes via the button so the
  default Tab proceeds from the picker's position.
- `aria-label` carries the consumer-supplied accessible name.
- Default labels title-case the slug.

---

Lily™ and Lily Design System™ are trademarks.
