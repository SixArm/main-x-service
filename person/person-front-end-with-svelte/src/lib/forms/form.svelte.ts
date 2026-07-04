// Tiny reactive form helper built on Svelte 5 runes. Holds value,
// per-field errors, submitting flag, and a submit-level error string.

/** Map of field name → validation error message. Empty means valid. */
export type FieldErrors = Record<string, string>;

/** Configuration passed to {@link createForm}. */
export interface CreateFormArgs<T> {
  /** Initial form value; deep-cloned so the caller's object is untouched. */
  initial: T;
  /** Optional synchronous validator returning field errors (empty = ok). */
  validate?: (value: T) => FieldErrors;
  /** Submit handler; its thrown error message becomes `submitError`. */
  onSubmit: (value: T) => Promise<void> | void;
}

/**
 * Reactive form handle returned by {@link createForm}. Getters expose
 * rune-backed state so consuming components stay reactive; methods mutate
 * that state.
 */
export interface FormState<T> {
  /** Current form value. */
  readonly value: T;
  /** Current per-field validation errors. */
  readonly errors: FieldErrors;
  /** Whether a submit is in flight. */
  readonly submitting: boolean;
  /** Submit-level error message (e.g. API failure), or `null`. */
  readonly submitError: string | null;
  /** Replace the entire value. */
  setValue(next: T): void;
  /** Shallow-merge a partial patch into the value. */
  update(patch: Partial<T>): void;
  /** Set a single field's error message. */
  setError(field: string, message: string): void;
  /** Clear a single field's error message. */
  clearError(field: string): void;
  /** Validate then run `onSubmit`; see {@link createForm}. */
  submit(): Promise<void>;
  /** Restore the value to its initial clone and clear all errors. */
  reset(): void;
}

/**
 * Create a tiny reactive form controller on Svelte 5 runes.
 *
 * Why: centralizes the value / errors / submitting / submit-error state and
 * the validate-then-submit flow so each page's form doesn't re-implement it.
 *
 * @typeParam T The form value shape.
 * @param args Initial value, optional validator, and submit handler.
 * @returns A {@link FormState} handle to bind to and drive from markup.
 */
export function createForm<T>(args: CreateFormArgs<T>): FormState<T> {
  // structuredClone isolates internal state from the caller's `initial`.
  let value = $state<T>(structuredClone(args.initial));
  let errors = $state<FieldErrors>({});
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  // Keep a pristine clone so reset() can restore the original value.
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
    // Reassign (not mutate) so Svelte's deep reactivity observes the change.
    update(patch: Partial<T>) {
      value = { ...value, ...patch };
    },
    setError(field: string, message: string) {
      errors = { ...errors, [field]: message };
    },
    clearError(field: string) {
      // Copy-and-delete to produce a new object reference for reactivity.
      const next = { ...errors };
      delete next[field];
      errors = next;
    },
    async submit() {
      submitError = null;
      // Run validation first; abort submit if any field error exists.
      const validationErrors = args.validate ? args.validate(value) : {};
      errors = validationErrors;
      if (Object.keys(validationErrors).length > 0) return;
      submitting = true;
      try {
        await args.onSubmit(value);
      } catch (err) {
        // Surface any thrown error (e.g. ApiError) as the submit error.
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
