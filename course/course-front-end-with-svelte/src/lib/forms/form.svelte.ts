// Tiny reactive form helper built on Svelte 5 runes. Holds value,
// per-field errors, submitting flag, and a submit-level error string.

/** Map of field name → human-readable error message for that field. */
export type FieldErrors = Record<string, string>;

/**
 * Configuration for {@link createForm}.
 *
 * @typeParam T - The form's value model.
 * @property initial - Starting value; deep-cloned so the original is never mutated.
 * @property validate - Optional synchronous validator returning field errors
 *   ({} ⇒ valid). Run on every submit before `onSubmit`.
 * @property onSubmit - Async submit handler invoked only when validation passes.
 */
export interface CreateFormArgs<T> {
    initial: T;
    validate?: (value: T) => FieldErrors;
    onSubmit: (value: T) => Promise<void> | void;
}

/**
 * Reactive form handle returned by {@link createForm}. The `value`,
 * `errors`, `submitting`, and `submitError` getters are backed by
 * Svelte runes, so reading them in a component sets up reactivity.
 *
 * @typeParam T - The form's value model.
 */
export interface FormState<T> {
    readonly value: T;
    readonly errors: FieldErrors;
    readonly submitting: boolean;
    readonly submitError: string | null;
    /** Replace the entire value. */
    setValue(next: T): void;
    /** Shallow-merge a partial patch into the value. */
    update(patch: Partial<T>): void;
    /** Set the error message for one field. */
    setError(field: string, message: string): void;
    /** Clear the error for one field, if any. */
    clearError(field: string): void;
    /** Validate then submit; never rejects (failures surface via `submitError`). */
    submit(): Promise<void>;
    /** Restore the value to its initial snapshot and clear all errors. */
    reset(): void;
}

/**
 * Build a tiny reactive form controller on Svelte 5 runes, tracking
 * value, per-field errors, a submitting flag, and a submit-level error
 * string. The model is deep-cloned on construction and on reset so the
 * caller's `initial` object is never mutated.
 *
 * @typeParam T - The form's value model.
 * @param args - Initial value, optional validator, and submit handler.
 * @returns A {@link FormState} handle for use in a component.
 */
export function createForm<T>(args: CreateFormArgs<T>): FormState<T> {
    // Clone so external mutation of `args.initial` can't leak in, and
    // so `value` is independently mutable reactive state.
    let value = $state<T>(structuredClone(args.initial));
    let errors = $state<FieldErrors>({});
    let submitting = $state(false);
    let submitError = $state<string | null>(null);
    // Separate pristine snapshot retained for reset().
    const initial = structuredClone(args.initial);

    return {
        get value() { return value; },
        get errors() { return errors; },
        get submitting() { return submitting; },
        get submitError() { return submitError; },
        setValue(next: T) { value = next; },
        update(patch: Partial<T>) { value = { ...value, ...patch }; },
        // Reassign (not mutate) so Svelte's rune tracking sees the change.
        setError(field: string, message: string) { errors = { ...errors, [field]: message }; },
        clearError(field: string) {
            const next = { ...errors };
            delete next[field];
            errors = next;
        },
        async submit() {
            submitError = null;
            // Validate first; abort the submit if any field error exists.
            const validationErrors = args.validate ? args.validate(value) : {};
            errors = validationErrors;
            if (Object.keys(validationErrors).length > 0) return;
            submitting = true;
            try {
                await args.onSubmit(value);
            } catch (err) {
                // Surface submit failures (e.g. API errors) as a banner
                // string rather than letting the rejection escape.
                submitError = err instanceof Error ? err.message : String(err);
            } finally {
                // Always clear the flag, even on failure, so the button re-enables.
                submitting = false;
            }
        },
        reset() {
            value = structuredClone(initial);
            errors = {};
            submitError = null;
        },
    };
}
