/// <references types="houdini-svelte">

/** @type {import('houdini').ConfigFile} */
const config = {
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
			type: 'String'
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
	} ,features: {
        imperativeCache: true,
    }
};

export default config;
