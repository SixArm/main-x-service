// Tiny reactive form helper built on Svelte 5 runes. Holds value,
// per-field errors, submitting flag, and a submit-level error string.

/** Map of field name → human-readable error message for that field. */
export type FieldErrors = Record<string, string>;

/**
 * Configuration for {@link createForm}.
 *
 * @typeParam T - The form's value shape.
 * @property initial - Starting value (deep-cloned, so the caller's object
 *   is never mutated and `reset()` can restore it).
 * @property validate - Optional synchronous validator returning field errors;
 *   a non-empty result blocks submission.
 * @property onSubmit - Async (or sync) action invoked with the valid value.
 */
export interface CreateFormArgs<T> {
  initial: T;
  validate?: (value: T) => FieldErrors;
  onSubmit: (value: T) => Promise<void> | void;
}

/**
 * Reactive form handle returned by {@link createForm}. The read-only
 * getters are backed by Svelte 5 runes, so reading them inside markup
 * subscribes the component to changes.
 *
 * @typeParam T - The form's value shape.
 */
export interface FormState<T> {
  /** Current form value (reactive). */
  readonly value: T;
  /** Current per-field validation errors (reactive). */
  readonly errors: FieldErrors;
  /** True while an `onSubmit` is in flight (reactive). */
  readonly submitting: boolean;
  /** Submit-level error message (e.g. thrown by `onSubmit`), or null. */
  readonly submitError: string | null;
  /** Replace the entire value. */
  setValue(next: T): void;
  /** Shallow-merge a partial patch into the value. */
  update(patch: Partial<T>): void;
  /** Set a single field's error message. */
  setError(field: string, message: string): void;
  /** Clear a single field's error message. */
  clearError(field: string): void;
  /** Validate then run `onSubmit`; captures thrown errors in `submitError`. */
  submit(): Promise<void>;
  /** Restore the initial value and clear all errors. */
  reset(): void;
}

/**
 * Create a small reactive form controller backed by Svelte 5 runes.
 *
 * Centralises value, per-field errors, an in-flight flag, and submit-level
 * error handling so components don't re-implement this each time. The
 * initial value is deep-cloned both for the live state and for `reset()`,
 * keeping the caller's object immutable.
 *
 * @typeParam T - The form's value shape.
 * @param args - Initial value, optional validator, and submit handler.
 * @returns A {@link FormState} handle.
 */
export function createForm<T>(args: CreateFormArgs<T>): FormState<T> {
  // Clone so external mutation of args.initial can't leak into form state.
  let value = $state<T>(structuredClone(args.initial));
  let errors = $state<FieldErrors>({});
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  // Separate pristine snapshot retained for reset().
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
    // Reassign (rather than mutate) so the rune notices the change.
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
      // Validate first; surface all field errors and bail before submitting.
      const validationErrors = args.validate ? args.validate(value) : {};
      errors = validationErrors;
      if (Object.keys(validationErrors).length > 0) return;
      submitting = true;
      try {
        await args.onSubmit(value);
      } catch (err) {
        // Any thrown error becomes the submit-level banner message.
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
