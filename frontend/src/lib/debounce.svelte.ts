/** Delays a reactive value so keystrokes don't each trigger a server round-trip. */
export function debounced<T>(get: () => T, ms = 300) {
	let value = $state(get());

	$effect(() => {
		const next = get();
		if (next === value) return;

		const id = setTimeout(() => (value = next), ms);
		return () => clearTimeout(id);
	});

	return {
		get current() {
			return value;
		}
	};
}
