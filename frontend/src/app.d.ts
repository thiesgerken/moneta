// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
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
