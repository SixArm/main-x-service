// See https://svelte.dev/docs/kit/types#app
// SvelteKit ambient type augmentations for this site. Static, unauthenticated
// docs site — no App.Locals/Platform of note yet.
declare global {
  namespace App {
    interface Error {}
    interface Locals {}
    interface PageData {}
    interface PageState {}
    interface Platform {}
  }
}

export {};
