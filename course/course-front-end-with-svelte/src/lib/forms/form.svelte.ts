// Tiny reactive form helper built on Svelte 5 runes. Holds value,
// per-field errors, submitting flag, and a submit-level error string.

export type FieldErrors = Record<string, string>;

export interface CreateFormArgs<T> {
    initial: T;
    validate?: (value: T) => FieldErrors;
    onSubmit: (value: T) => Promise<void> | void;
}

export interface FormState<T> {
    readonly value: T;
    readonly errors: FieldErrors;
    readonly submitting: boolean;
    readonly submitError: string | null;
    setValue(next: T): void;
    update(patch: Partial<T>): void;
    setError(field: string, message: string): void;
    clearError(field: string): void;
    submit(): Promise<void>;
    reset(): void;
}

export function createForm<T>(args: CreateFormArgs<T>): FormState<T> {
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
