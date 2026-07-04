// Tiny reactive form helper built on Svelte 5 runes. Holds value,
// per-field errors, submitting flag, and a submit-level error string.

/** Map of field name → validation error message. */
export type FieldErrors = Record<string, string>;

/**
 * Configuration for {@link createForm}.
 * @typeParam T - The form's value shape.
 */
export interface CreateFormArgs<T> {
  /** Initial value; deep-cloned so the form owns its own copy. */
  initial: T;
  /** Optional synchronous validator returning field errors. */
  validate?: (value: T) => FieldErrors;
  /** Called with the value on a successful (error-free) submit. */
  onSubmit: (value: T) => Promise<void> | void;
}

/**
 * Reactive form handle returned by {@link createForm}. The `value`,
 * `errors`, `submitting`, and `submitError` getters are backed by Svelte 5
 * runes, so reading them in markup tracks changes automatically.
 * @typeParam T - The form's value shape.
 */
export interface FormState<T> {
  /** Current form value (reactive). */
  readonly value: T;
  /** Current per-field errors (reactive). */
  readonly errors: FieldErrors;
  /** Whether a submit is in flight (reactive). */
  readonly submitting: boolean;
  /** Submit-level error message, or null (reactive). */
  readonly submitError: string | null;
  /** Replace the whole value. */
  setValue(next: T): void;
  /** Shallow-merge a partial patch into the value. */
  update(patch: Partial<T>): void;
  /** Set a single field's error message. */
  setError(field: string, message: string): void;
  /** Clear a single field's error. */
  clearError(field: string): void;
  /** Validate then, if clean, run `onSubmit`; captures thrown errors. */
  submit(): Promise<void>;
  /** Restore the value to `initial` and clear all errors. */
  reset(): void;
}

/**
 * Create a small reactive form helper built on Svelte 5 runes — value,
 * per-field errors, an in-flight flag, and a submit-level error string.
 *
 * Kept deliberately tiny (no schema lib) and per-project per the drift
 * policy. The initial value is `structuredClone`d twice: once for the live
 * `value` and once stored for {@link FormState.reset}, so external mutation
 * of `args.initial` can't leak in.
 *
 * @returns A {@link FormState} handle to drive the form from markup.
 */
export function createForm<T>(args: CreateFormArgs<T>): FormState<T> {
  let value = $state<T>(structuredClone(args.initial));
  let errors = $state<FieldErrors>({});
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  const initial = structuredClone(args.initial);

  return {
    get value() {
      return value;
    },
    get errors() {
      return errors;
    },
    get submitting() {
      return submitting;
    },
    get submitError() {
      return submitError;
    },
    setValue(next: T) {
      value = next;
    },
    update(patch: Partial<T>) {
      value = { ...value, ...patch };
    },
    setError(field: string, message: string) {
      errors = { ...errors, [field]: message };
    },
    clearError(field: string) {
      const next = { ...errors };
      delete next[field];
      errors = next;
    },
    async submit() {
      submitError = null;
      // Run validation first; publish errors and bail before submitting.
      const validationErrors = args.validate ? args.validate(value) : {};
      errors = validationErrors;
      if (Object.keys(validationErrors).length > 0) return;
      submitting = true;
      try {
        await args.onSubmit(value);
      } catch (err) {
        // Surface any thrown error (e.g. ApiError) as a submit banner.
        submitError = err instanceof Error ? err.message : String(err);
      } finally {
        // Always clear the in-flight flag, success or failure.
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
