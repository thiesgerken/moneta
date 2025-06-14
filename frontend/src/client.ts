import { HoudiniClient } from '$houdini';

const isServer = typeof window === 'undefined';

const graphqlUrl = isServer ? process.env.GRAPHQL_API_URL : '/graphql';
console.log(`Using GraphQL URL: ${graphqlUrl}`);

export default new HoudiniClient({
	url: graphqlUrl

	// uncomment this to configure the network call (for things like authentication)
	// for more information, please visit here: https://www.houdinigraphql.com/guides/authentication
	// fetchParams({ session }) {
	//     return {
	//         headers: {
	//             Authorization: `Bearer ${session.token}`,
	//         }
	//     }
	// }
});
