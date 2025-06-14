import { graphql } from '$houdini';
import { fail } from '@sveltejs/kit';
import type { Actions } from './$types';

export const actions = {
	login: async (event) => {
		const data = await event.request.formData();

		const username = data.get('username')?.toString();
		const password = data.get('password')?.toString();

		if (!username || !password) {
			return fail(403);
		}

		const loginMutation = graphql(`
			mutation LoginMutation($name: String!, $password: String!) {
				users {
					login(name: $name, password: $password) {
						id
					}
				}
			}
		`);

		return await loginMutation.mutate({ name: username, password }, { event });
	}
} satisfies Actions;
