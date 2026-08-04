/// <references types="houdini-svelte">

/** @type {import('houdini').ConfigFile} */
const config = {
	// `url` ships in the client bundle, so it stays relative and is routed to the
	// backend by the vite proxy in dev and by traefik in production
	url: '/graphql',
	schemaPath: './schema.graphql',
	watchSchema: {
		url: 'http://localhost:8181/graphql'
	},
	runtimeDir: '.houdini',
	plugins: {
		'houdini-svelte': {
			forceRunesMode: true
		}
	},
	scalars: {
		ObjectIdWrapper: {
			type: 'string'
		},
		DateTime: {
			type: 'Date',
			unmarshal(val) {
				return val ? new Date(val) : null;
			},
			marshal(date) {
				return date && date.toISOString();
			}
		}
	}
};

export default config;
