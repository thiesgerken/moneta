// Takes the README's screenshots from a running dev server, light and dark, with headless
// Chrome over the DevTools protocol. No dependencies: node 22+ has fetch and WebSocket built in.
//
//   node docs/screenshots/take.mjs [http://localhost:5173]
//
// Chrome's own --screenshot cannot open a table row, and a fixed virtual-time budget either
// waits too long or catches a chart halfway through loading; this waits for the page to settle.
// README.md next to it says what has to be running first.

import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const BASE = process.argv[2] ?? 'http://localhost:5173';
const CHROME =
  process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const OUT = dirname(fileURLToPath(import.meta.url));
const PORT = 9333;

/// Each page, and the table rows to open on it (matched on the start of the row's text).
const PAGES = [
  { name: 'overview', path: '/' },
  { name: 'expenses', path: '/expenses' },
  { name: 'analysis', path: '/analysis?tab=history' },
  { name: 'accounts', path: '/accounts', open: ['Schulden bei Krösus'] },
  { name: 'rules', path: '/rules', open: ['REWE'] }
];

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function pageSocket() {
  for (let attempt = 0; attempt < 50; attempt++) {
    try {
      const response = await fetch(`http://127.0.0.1:${PORT}/json/new?about:blank`, {
        method: 'PUT'
      });
      return (await response.json()).webSocketDebuggerUrl;
    } catch {
      await sleep(200);
    }
  }
  throw new Error('Chrome did not come up');
}

/// A DevTools session: commands with replies, and every event to whoever listens.
async function connect(url) {
  const socket = new WebSocket(url);
  const pending = new Map();
  const listeners = new Set();
  let next = 0;

  socket.onmessage = ({ data }) => {
    const message = JSON.parse(data);
    const reply = pending.get(message.id);
    if (reply) {
      pending.delete(message.id);
      if (message.error) reply.reject(new Error(message.error.message));
      else reply.resolve(message.result);
    } else {
      for (const listener of [...listeners]) listener(message);
    }
  };
  await new Promise((resolve) => (socket.onopen = resolve));

  return {
    send: (method, params = {}) =>
      new Promise((resolve, reject) => {
        const id = ++next;
        pending.set(id, { resolve, reject });
        socket.send(JSON.stringify({ id, method, params }));
      }),
    once: (method) =>
      new Promise((resolve) => {
        const listener = (message) => {
          if (message.method !== method) return;
          listeners.delete(listener);
          resolve(message.params);
        };
        listeners.add(listener);
      }),
    listen: (listener) => listeners.add(listener),
    unlisten: (listener) => listeners.delete(listener),
    close: () => socket.close()
  };
}

/// Until nothing has been fetched for a moment and no spinner is left on the page.
async function settle(page) {
  const inFlight = new Set();
  let quietSince = Date.now();
  // requests only: the dev server's HMR socket chatters for as long as the page is open. By
  // id, because a redirect announces the same request twice and finishes it once.
  const track = ({ method, params }) => {
    if (method === 'Network.requestWillBeSent') inFlight.add(params.requestId);
    else if (method === 'Network.loadingFinished' || method === 'Network.loadingFailed')
      inFlight.delete(params.requestId);
    else return;
    quietSince = Date.now();
  };
  page.listen(track);
  for (let waited = 0; waited < 30_000; waited += 250) {
    await sleep(250);
    const { result } = await page.send('Runtime.evaluate', {
      expression: "!!document.querySelector('[role=status], .animate-spin')"
    });
    if (inFlight.size === 0 && !result.value && Date.now() - quietSince > 1500) break;
  }
  page.unlisten(track);
  // the charts animate in once their data is there
  await sleep(1200);
}

const profile = mkdtempSync(join(tmpdir(), 'moneta-screenshots-'));
const chrome = spawn(
  CHROME,
  [
    '--headless=new',
    '--hide-scrollbars',
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`
  ],
  { stdio: 'ignore' }
);

try {
  const page = await connect(await pageSocket());
  await page.send('Network.enable');
  await page.send('Page.enable');
  await page.send('Emulation.setDeviceMetricsOverride', {
    width: 1280,
    height: 800,
    deviceScaleFactor: 2,
    mobile: false
  });

  for (const scheme of ['light', 'dark']) {
    await page.send('Emulation.setEmulatedMedia', {
      features: [{ name: 'prefers-color-scheme', value: scheme }]
    });
    for (const { name, path, open = [] } of PAGES) {
      const loaded = page.once('Page.loadEventFired');
      await page.send('Page.navigate', { url: BASE + path });
      await loaded;
      await settle(page);
      for (const start of open) {
        await page.send('Runtime.evaluate', {
          expression: `[...document.querySelectorAll('tbody tr')]
            .find((row) => row.innerText.trim().startsWith(${JSON.stringify(start)}))
            ?.click()`
        });
        await sleep(400);
      }
      const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(join(OUT, `${name}-${scheme}.png`), Buffer.from(data, 'base64'));
      console.log(`${name}-${scheme}.png`);
    }
  }
  page.close();
} finally {
  chrome.kill();
  await sleep(300);
  rmSync(profile, { recursive: true, force: true });
}
