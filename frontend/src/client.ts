import { HoudiniClient } from '$houdini';

// the request url moved to houdini.config.js in houdini 2 and is no longer accepted here
export default new HoudiniClient({
	fetchParams() {
		return {
			credentials: 'include'
		};
	}
});
