// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title>.
export function load() {
    return { title: 'Tour · Case Tracking' };
}
