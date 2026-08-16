// Posts the coverage table on a pull request, editing its own previous comment rather than
// adding another one on every push. A run of ten pushes should leave one comment saying where
// the branch stands, not ten saying where it stood.
//
// Node rather than a shell pipeline because the runner image is only guaranteed to have what
// setup-node put there -- no jq, no python. `fetch` is built in.
//
// Reads:
//   backend-coverage.json   `cargo llvm-cov --json --summary-only`
//   frontend-coverage.json  vitest's json-summary reporter
// Environment: GITEA_TOKEN, GITEA_API (server url), GITEA_REPO (owner/name), PR_NUMBER,
//              SQL_ASSERTIONS

import { readFileSync, readdirSync } from 'node:fs';

/// Both artifacts unpack into this directory, and an artifact built from a single path keeps
/// only that file's basename — so what arrives here depends on how the upload was spelled, and
/// getting it wrong shows up as an ENOENT with no clue in it. Say what was there instead.
function read(file) {
	try {
		return JSON.parse(readFileSync(file, 'utf8'));
	} catch (error) {
		if (error.code !== 'ENOENT') throw error;
		throw new Error(`${file} is not among the downloaded artifacts: ${readdirSync('.').join(', ')}`);
	}
}

// What identifies our own comment among all of them. Invisible in the rendered markdown.
const MARKER = '<!-- moneta-coverage -->';

const pct = (covered, total) =>
	total === 0 ? '—' : `${((covered / total) * 100).toFixed(1)}% (${covered}/${total})`;

/** Vitest reports per file, so the two halves are split on the extension of the key. */
function splitFrontend(summary) {
	const acc = {
		ts: { lines: [0, 0], funcs: [0, 0] },
		svelte: { lines: [0, 0], funcs: [0, 0] }
	};

	for (const [file, m] of Object.entries(summary)) {
		if (file === 'total') continue;
		const bucket = file.endsWith('.svelte') ? acc.svelte : acc.ts;
		bucket.lines[0] += m.lines.covered;
		bucket.lines[1] += m.lines.total;
		bucket.funcs[0] += m.functions.covered;
		bucket.funcs[1] += m.functions.total;
	}

	return acc;
}

function body() {
	const backend = read('backend-coverage.json').data[0].totals;
	const front = splitFrontend(read('frontend-coverage.json'));

	const rows = [
		// llvm-cov says {covered, count}; the split above already produces [covered, total]
		['backend (rust)', [backend.lines.covered, backend.lines.count],
			[backend.functions.covered, backend.functions.count]],
		['frontend modules (`.ts`)', front.ts.lines, front.ts.funcs],
		['frontend components (`.svelte`)', front.svelte.lines, front.svelte.funcs]
	];

	const table = rows
		.map(([label, lines, funcs]) => `| ${label} | ${pct(...lines)} | ${pct(...funcs)} |`)
		.join('\n');

	const sql = process.env.SQL_ASSERTIONS;

	return `${MARKER}
## Coverage

| | lines | functions |
| --- | --- | --- |
${table}

${sql} SQL assertions run against a freshly migrated database on top of this — the rendered
views, the CHECK constraints and the analysis functions carry the money maths, and none of
that shows up in the Rust figure above.`;
}

const api = `${process.env.GITEA_API}/api/v1/repos/${process.env.GITEA_REPO}`;
const headers = {
	Authorization: `token ${process.env.GITEA_TOKEN}`,
	'Content-Type': 'application/json'
};

const call = async (method, path, payload) => {
	const response = await fetch(api + path, {
		method,
		headers,
		body: payload === undefined ? undefined : JSON.stringify(payload)
	});
	if (!response.ok) {
		throw new Error(`${method} ${path} -> ${response.status} ${await response.text()}`);
	}
	return response.json();
};

const comments = await call('GET', `/issues/${process.env.PR_NUMBER}/comments`);
const mine = comments.find((c) => c.body?.startsWith(MARKER));

if (mine) {
	await call('PATCH', `/issues/comments/${mine.id}`, { body: body() });
	console.log(`updated comment ${mine.id}`);
} else {
	const created = await call('POST', `/issues/${process.env.PR_NUMBER}/comments`, { body: body() });
	console.log(`created comment ${created.id}`);
}
