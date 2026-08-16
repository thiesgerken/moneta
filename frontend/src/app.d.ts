// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	/// The commit the bundle was built from, or '' outside a CI build. Substituted by vite.
	const __GIT_SHA__: string;

	namespace App {
		// houdini declares this empty for apps to fill in; `code` is what the backend puts on an
		// auth error (require_user in graphql/root.rs)
		interface GraphQLErrorExtensions {
			code?: string;
		}
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
