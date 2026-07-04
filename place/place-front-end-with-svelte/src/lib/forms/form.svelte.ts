// Tiny reactive form helper built on Svelte 5 runes. Holds value,
// per-field errors, submitting flag, and a submit-level error string.

/** Map of field name → error message; empty means the form is valid. */
export type FieldErrors = Record<string, string>;

/**
 * Configuration for {@link createForm}.
 * @typeParam T - The form's value shape.
 */
export interface CreateFormArgs<T> {
  /** Initial value; deep-cloned so the original is never mutated. */
  initial: T;
  /** Optional synchronous validator returning field errors. */
  validate?: (value: T) => FieldErrors;
  /** Submit handler invoked only when validation passes. */
  onSubmit: (value: T) => Promise<void> | void;
}

/**
 * Reactive form handle returned by {@link createForm}. Read accessors are
 * backed by Svelte `$state`, so reading them in markup is reactive.
 * @typeParam T - The form's value shape.
 */
export interface FormState<T> {
  /** Current form value (reactive). */
  readonly value: T;
  /** Current per-field errors (reactive). */
  readonly errors: FieldErrors;
  /** True while {@link FormState.submit} is awaiting `onSubmit`. */
  readonly submitting: boolean;
  /** Submit-level error message, or `null` (reactive). */
  readonly submitError: string | null;
  /** Replace the whole value. */
  setValue(next: T): void;
  /** Shallow-merge a partial patch into the value. */
  update(patch: Partial<T>): void;
  /** Set a single field's error message. */
  setError(field: string, message: string): void;
  /** Clear a single field's error. */
  clearError(field: string): void;
  /** Validate then (if clean) run `onSubmit`, tracking submitting/error. */
  submit(): Promise<void>;
  /** Restore the initial value and clear all errors. */
  reset(): void;
}

/**
 * Create a minimal reactive form controller over Svelte 5 runes.
 *
 * Holds the value, per-field errors, a submitting flag, and a submit-level
 * error. {@link FormState.submit} short-circuits when validation produces
 * any errors and converts thrown errors into `submitError` rather than
 * rejecting, so callers can simply `await form.submit()`.
 * @typeParam T - The form's value shape.
 * @param args - Initial value, optional validator, and submit handler.
 * @returns A {@link FormState} handle.
 */
export function createForm<T>(args: CreateFormArgs<T>): FormState<T> {
  // Clone so external mutation of `args.initial` can't leak into state.
  let value = $state<T>(structuredClone(args.initial));
  let errors = $state<FieldErrors>({});
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  // Pristine copy retained for reset().
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
    // Reassign (not mutate) so Svelte sees a new object and re-renders.
    setError(field: string, message: string) {
      errors = { ...errors, [field]: message };
    },
    clearError(field: string) {
      const next = { ...errors };
      delete next[field];
      errors = next; // reassign to trigger reactivity
    },
    async submit() {
      submitError = null;
      // Run validation; an empty map means "valid".
      const validationErrors = args.validate ? args.validate(value) : {};
      errors = validationErrors;
      if (Object.keys(validationErrors).length > 0) return; // block submit
      submitting = true;
      try {
        await args.onSubmit(value);
      } catch (err) {
        // Surface failures as a form-level message instead of rejecting.
        submitError = err instanceof Error ? err.message : String(err);
      } finally {
        submitting = false; // always clear, even on error
      }
    },
    reset() {
      value = structuredClone(initial);
      errors = {};
      submitError = null;
    },
  };
}
