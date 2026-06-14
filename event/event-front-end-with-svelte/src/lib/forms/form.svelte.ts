// Tiny reactive form helper built on Svelte 5 runes. Holds value,
// per-field errors, submitting flag, and a submit-level error string.

/** Map of field name → validation error message for that field. */
export type FieldErrors = Record<string, string>;

/**
 * Configuration for {@link createForm}.
 * @typeParam T - The form's value shape.
 */
export interface CreateFormArgs<T> {
    /** Initial value; deep-cloned so the original is never mutated. */
    initial: T;
    /** Optional synchronous validator returning per-field errors (empty = valid). */
    validate?: (value: T) => FieldErrors;
    /** Submit handler invoked with the current value once validation passes. */
    onSubmit: (value: T) => Promise<void> | void;
}

/**
 * Reactive form controller returned by {@link createForm}. The getters are
 * backed by Svelte runes so reads inside markup stay reactive.
 * @typeParam T - The form's value shape.
 */
export interface FormState<T> {
    /** Current form value (reactive). */
    readonly value: T;
    /** Current per-field validation errors (reactive). */
    readonly errors: FieldErrors;
    /** True while {@link FormState.submit} is awaiting `onSubmit` (reactive). */
    readonly submitting: boolean;
    /** Submit-level error message (e.g. server failure), or null (reactive). */
    readonly submitError: string | null;
    /** Replace the entire value. */
    setValue(next: T): void;
    /** Shallow-merge a partial patch into the value. */
    update(patch: Partial<T>): void;
    /** Set a single field's error message. */
    setError(field: string, message: string): void;
    /** Clear a single field's error message. */
    clearError(field: string): void;
    /** Validate then run `onSubmit`; captures thrown errors into `submitError`. */
    submit(): Promise<void>;
    /** Restore the value to a clone of `initial` and clear all errors. */
    reset(): void;
}

/**
 * Create a small reactive form controller built on Svelte 5 runes,
 * tracking value, per-field errors, a submitting flag, and a submit-level
 * error string — without any global store.
 *
 * @typeParam T - The form's value shape.
 * @param args - Initial value, optional validator, and submit handler.
 * @returns A {@link FormState} bound to its own reactive state.
 */
export function createForm<T>(args: CreateFormArgs<T>): FormState<T> {
    // structuredClone keeps the caller's `initial` immutable across edits/resets.
    let value = $state<T>(structuredClone(args.initial));
    let errors = $state<FieldErrors>({});
    let submitting = $state(false);
    let submitError = $state<string | null>(null);
    const initial = structuredClone(args.initial);

    return {
        get value() { return value; },
        get errors() { return errors; },
        get submitting() { return submitting; },
        get submitError() { return submitError; },
        setValue(next: T) { value = next; },
        update(patch: Partial<T>) { value = { ...value, ...patch }; },
        setError(field: string, message: string) { errors = { ...errors, [field]: message }; },
        clearError(field: string) {
            const next = { ...errors };
            delete next[field];
            errors = next;
        },
        async submit() {
            submitError = null;
            const validationErrors = args.validate ? args.validate(value) : {};
            errors = validationErrors;
            // Abort before touching the network if any field is invalid.
            if (Object.keys(validationErrors).length > 0) return;
            submitting = true;
            try {
                await args.onSubmit(value);
            } catch (err) {
                submitError = err instanceof Error ? err.message : String(err);
            } finally {
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
