# Screenshots

The README's screenshots, light and dark, always of the demo household in
[`docs/demo/rudolph.json`](../demo/rudolph.json) and never of real data: Rudolph and Krösus share
a flat, a child in Kita and a pair of debt accounts, and everything in their books is invented.

| File | Page | What it shows |
| --- | --- | --- |
| `overview-*.png` | `/` | net worth and its curve, the to-do cards (changes by Krösus, unchecked REWE purchases, last month to go through), the running month's categories and the activity since the last visit |
| `expenses-*.png` | `/expenses` | the expense list: an incomplete online order and unchecked groceries on top, shared bookings across Girokonto and the debt account, rent and Kita over their whole month |
| `analysis-*.png` | `/analysis?tab=history` | income and spending per month over the last year, and the monthly overview with wealth at either end |
| `accounts-*.png` | `/accounts` | Rudolph's accounts, the debt account opened up to show it mirrors Krösus' with the sign flipped |
| `rules-*.png` | `/rules` | periodic and automation rules side by side, the REWE rule opened |

## Retaking them

1. **Bring the demo up to date.** It ends on the day it was generated, and the start page talks
   about "this month" and "since your last visit", so regenerate it for today first:

   ```sh
   python3 docs/demo/generate.py --until "$(date +%F)"
   ```

   Do not commit that one. The committed file is pinned to a fixed date so that it can be
   reproduced byte for byte; when the export format changes, regenerate it without `--until`.

2. **Import it into an empty database.** `moneta import` refuses a database that already holds
   user data, and the first page load moves the visit marker the start page dates its news
   from, so start from scratch each time:

   ```sh
   docker run -d --name moneta-demo -e POSTGRES_USER=moneta -e POSTGRES_PASSWORD=moneta \
       -e POSTGRES_DB=moneta -p 127.0.0.1:5498:5432 postgres:18-alpine
   cd backend
   export MONETA__DATABASE__URL=postgres://moneta:moneta@localhost:5498/moneta
   cargo run -- import < ../docs/demo/rudolph.json
   ```

3. **Run the backend signing everybody in as Rudolph's proxy would.** The demo users have no
   password; the dev server below vouches for Rudolph with `Remote-User`, and it connects from
   127.0.0.1:

   ```sh
   MONETA__API__TRUSTED_HEADER_AUTHENTICATION=true MONETA__API__TRUSTED_PROXIES=127.0.0.1/32 \
       cargo run -- serve
   ```

   Restart it after a re-import: the analysis is cached in the process for ten minutes.

4. **Run the frontend through a proxy that adds the header.** Save this as
   `frontend/vite.screenshots.config.ts`, which is gitignored: nothing else should sign in like
   that.

   ```ts
   import { defineConfig, mergeConfig } from 'vite';
   import base from './vite.config';

   export default mergeConfig(
     base,
     defineConfig({
       server: {
         proxy: {
           '/graphql': { target: 'http://127.0.0.1:8181', headers: { 'Remote-User': 'Rudolph' } }
         }
       }
     })
   );
   ```

   ```sh
   cd frontend && npx vite dev --config vite.screenshots.config.ts
   ```

5. **Take them.** [`take.mjs`](take.mjs) drives headless Chrome over the DevTools protocol (node
   22 or newer, no dependencies): every page at 1280×800 with device scale factor 2, once with
   `prefers-color-scheme: light` and once with `dark`, which is all the theme follows. It waits
   until the page has stopped fetching, opens the rows listed in its `PAGES`, and writes the PNGs
   next to itself:

   ```sh
   node docs/screenshots/take.mjs            # CHROME=/path/to/chrome if it is not the macOS default
   ```

6. **Look at every one before committing it**: no empty chart, no error, no spinner, nothing
   that is not from the demo.
