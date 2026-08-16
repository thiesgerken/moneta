/// What a table is currently filtered by, and how to undo each part.
///
/// Lives here rather than in `FilterChips.svelte`: a type exported from a component's instance
/// script only resolves because the Svelte language tooling hoists it, and `export` there means
/// "prop" to everything else.
export type ActiveFilter = { label: string; color?: string | null; clear: () => void };

/// Adding or removing one option of a multiple-choice filter. Copied into four tables before it
/// was worth a home.
export const toggleOption = (selected: string[], id: string) =>
	selected.includes(id) ? selected.filter((other) => other !== id) : [...selected, id];
