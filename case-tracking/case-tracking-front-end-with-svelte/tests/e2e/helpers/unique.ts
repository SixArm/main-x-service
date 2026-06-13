// Unique-name helpers so concurrent / repeated test runs don't collide
// on upstream resources (folder titles, place names, etc).

export function unique(prefix: string): string {
    const stamp = Date.now().toString(36);
    const rand = Math.random().toString(36).slice(2, 6);
    return `${prefix}-${stamp}-${rand}`;
}
