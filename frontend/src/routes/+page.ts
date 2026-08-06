import { redirect } from '@sveltejs/kit';

/// The dashboard was one tab strip holding a single tab, next to another holding six. Its one
/// tab lives in that strip now, and the root is a way in rather than a page of its own.
export function load() {
	redirect(307, '/analysis');
}
