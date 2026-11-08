<p align="center">
  <img alt="Moneta" src="frontend/static/brand/lockup.svg#gh-light-mode-only" width="296" height="64">
  <img alt="Moneta" src="frontend/static/brand/lockup-dark.svg#gh-dark-mode-only" width="296" height="64">
</p>
<p align="center"><strong>A household's money, kept together: who paid, who owes, and what it cost each of you.</strong></p>

<p align="center">Expenses shared across several people's accounts, filed by when the money moved and by what it paid for, and booked from bank lines by rules.</p>

<p align="center">
  <a href="#-getting-started">🚀 Getting started</a>
  ·
  <a href="#-screenshots">📸 Screenshots</a>
  ·
  <a href="#-configuration">🔧 Configuration</a>
  ·
  <a href="docs/README.md">📚 Docs</a>
</p>

Moneta records what a household spends and owns. Several people share it: some accounts belong
to two of them, one purchase can move money on more than one account, and a bill one of them
paid is half the other's. Moneta keeps all of that in one set of books and shows each person
their own view of it, from net worth down to a single receipt. It runs on your own server as a
Rust backend with a GraphQL API over PostgreSQL and a SvelteKit frontend. The interface is
German.

## ✨ Highlights

- 👥 **One set of books, a view for each.** An expense belongs to nobody; it shows up for whoever
  owns the accounts it touches, at their share. A purchase split three ways is written as
  fractions of one amount, and a debt between two people is a pair of mirrored accounts, each
  seeing the other's entry with the sign flipped.
- 📅 **When it was paid, and what it was for.** Every expense has the day the money moved and the
  period it covers. A yearly insurance paid in January spreads over the year, childcare debited
  on the 30th counts for the month it pays for, and the analysis switches between the two views.
- 🏦 **Bank lines booked by rules.** Automation rules match statement lines by text, date, amount
  and account and wrap them in a template: the shares, the categories, the period. Periodic
  rules write the rent and the savings plan on schedule. Re-delivered lines are recognised and
  skipped, unclaimed ones wait in a queue.
- 📊 **Analysis that adds up.** Net worth over time by account or by how quickly the money can be
  reached, income and spending per month and category, prepayments still to be used up, a
  statement per account and a calendar. Balances you record are evidence: the computed figures
  reconcile against them and show the difference.
- 🕓 **Nothing changes silently.** Every edit is kept as a before-and-after snapshot, shown as a
  diff on the expense and as a feed of what happened since your last visit.
- 🔑 **Sign-in that fits the setup.** Passwords, read or write API keys for scripts and agents,
  or a login proxy's `Remote-User` header.
- 📦 **Your data as one file.** `moneta export` writes everything except credentials as JSON,
  `moneta import` restores it into an empty database.

## 📸 Screenshots

<p align="center">
  <img alt="Start page" src="docs/screenshots/overview-light.png#gh-light-mode-only" width="1000">
  <img alt="Start page" src="docs/screenshots/overview-dark.png#gh-dark-mode-only" width="1000">
</p>

<p align="center"><em>The start page: net worth and its curve, what wants attention, this month's categories, and what happened since the last visit.</em></p>

<br>

<p align="center">
  <img alt="Expenses" src="docs/screenshots/expenses-light.png#gh-light-mode-only" width="1000">
  <img alt="Expenses" src="docs/screenshots/expenses-dark.png#gh-dark-mode-only" width="1000">
</p>

<p align="center"><em>Expenses: shared bookings across a current account and a debt account, rent and childcare over the month they pay for, and what still needs checking.</em></p>

<br>

<p align="center">
  <img alt="Analysis" src="docs/screenshots/analysis-light.png#gh-light-mode-only" width="1000">
  <img alt="Analysis" src="docs/screenshots/analysis-dark.png#gh-dark-mode-only" width="1000">
</p>

<p align="center"><em>Income and spending per month, and an overview that reconciles against the recorded balances.</em></p>

<br>

<p align="center">
  <img alt="Accounts" src="docs/screenshots/accounts-light.png#gh-light-mode-only" width="1000">
  <img alt="Accounts" src="docs/screenshots/accounts-dark.png#gh-dark-mode-only" width="1000">
</p>

<p align="center"><em>Accounts, with a debt account that mirrors the other person's.</em></p>

<br>

<p align="center">
  <img alt="Rules" src="docs/screenshots/rules-light.png#gh-light-mode-only" width="1000">
  <img alt="Rules" src="docs/screenshots/rules-dark.png#gh-dark-mode-only" width="1000">
</p>

<p align="center"><em>Periodic and automation rules side by side.</em></p>

All screenshots show the demo household of Rudolph and Krösus: two invented people and two
invented years of their books. [Retaking them](docs/screenshots/README.md) is scripted.

## 🚀 Getting started

The quickest way to see Moneta with something in it is to run it locally and import the demo
household. You need Docker (for PostgreSQL 18), Rust (rustup picks the pinned toolchain from
`rust-toolchain.toml`) and Node. With Nix, `nix develop` brings all of them except Docker.

In one terminal, from the top of the repository:

```sh
docker compose up -d postgres                       # PostgreSQL 18 on 127.0.0.1:5433
cd backend
cargo run -- import < ../docs/demo/rudolph.json     # into the still empty database
cargo run -- user --update Rudolph                  # imported users have no password yet
cargo run -- serve                                  # the API on :8181, GraphiQL at its root
```

`user --update` asks for the full name again (type `Rudolph`; an empty answer is stored as
empty), then for a password of at least 8 characters.

In a second terminal, again from the top:

```sh
cd frontend
npm ci && npm run dev                               # the UI on :5173, proxying /graphql to :8181
```

Open <http://localhost:5173> and sign in as Rudolph. The demo ends on the day it was generated.
To make it end today, generate a fresh one under a gitignored name and import that instead, into
an empty database:

```sh
python3 docs/demo/generate.py --until "$(date +%F)" --output demo-export.json
cd backend && cargo run -- import < ../demo-export.json
```

For your own books, start from an empty database and add yourself with
`cargo run -- user --add <name>`, which asks for a full name and a password of at least 8
characters.

The images are built with Nix, and [`docker-compose.yaml`](docker-compose.yaml) runs the whole
stack from them behind traefik; its registry is not public, so build them yourself first.
[Operations](docs/operations.md) has that, the periodic job, and what to change before the
stack holds real data.

## 🔧 Configuration

Settings come from environment variables prefixed `MONETA__`, nested with `__`
(`MONETA__DATABASE__URL`), or a `.env` file next to the backend. Defaults live in
[`backend/src/config.rs`](backend/src/config.rs).

| Variable | Default | Notes |
| --- | --- | --- |
| `MONETA__DATABASE__URL` | `postgres://moneta:moneta@localhost:5433/moneta` | matches the port docker-compose exposes |
| `MONETA__API__PORT` | `8181` | |
| `MONETA__API__ADDRESS` | `127.0.0.1` | |
| `MONETA__API__SECRET_KEY` | *(empty)* | signs the session cookie; `openssl rand -base64 32` |
| `MONETA__API__TRUSTED_HEADER_AUTHENTICATION` | `false` | sign people in by the `Remote-User` header a login proxy sets, see [signing in](#-signing-in) |
| `MONETA__API__TRUSTED_PROXIES` | *(empty)* | CIDRs, comma-separated (`10.42.0.0/16,10.0.0.5/32`), that `Remote-User` is believed from; required when the above is on |
| `MONETA__RENDITIER__URL` | *(empty)* | where the backend reaches [renditier](https://github.com/thiesgerken/renditier) for today's depot values; empty switches the integration off |
| `MONETA__RENDITIER__PUBLIC_URL` | *(empty)* | where a *browser* reaches it, for the link on a depot; falls back to `URL` |
| `MONETA__RENDITIER__TIMEOUT_SECONDS` | `5` | a page load waits on this, so it has to give up quickly |

**PostgreSQL 18 is a hard requirement**: ids default to `uuidv7()`, which does not exist before
it. Migrations run on their own before every subcommand except `schema`.

## 🔑 Signing in

- **Password.** `moneta user --add <name>` creates a user and asks for a full name and a
  password of at least 8 characters; `--update <name>` asks for both again and replaces them.
- **API keys** for scripts and agents: created under *API-Schlüssel* in the browser, read-only or
  read-write, sent as `Authorization: Bearer mk_…` to the same GraphQL endpoint. An explicit
  key always wins over any other credential, even when it is invalid. [API keys](docs/api.md)
  has examples and the rules.
- **A login proxy** (Authelia, oauth2-proxy, traefik forward auth, …) can sign people in with a
  `Remote-User` header naming an existing user, with
  `MONETA__API__TRUSTED_HEADER_AUTHENTICATION=true` and the proxy's addresses in
  `MONETA__API__TRUSTED_PROXIES`. The header is believed only from those addresses, checked
  against the TCP connection. **This is safe only when the backend is reachable solely through
  that proxy, and the proxy removes a `Remote-User` the client sent.** Details are in
  [operations](docs/operations.md#signing-in-through-a-login-proxy).

## 📦 Export and import

```sh
moneta export > moneta-export.json   # everything but credentials, as JSON on stdout
moneta import < moneta-export.json   # only into a database without user data
```

The file holds everything personal there is, which is why both are CLI-only and
`*-export.json` is gitignored. Import refuses a database that already holds user data, checks
the file before writing, and rolls back on any error; imported users get a password with
`moneta user --update <name>`. It complements a `pg_dump` rather than replacing it.
[Operations](docs/operations.md#export-and-import) has the details, and the rest of the
[command line](docs/operations.md#cli).

## 🛠️ Development

[`docs/`](docs/README.md) explains what the system is, starting with the
[data model](docs/concepts.md) and [what the app does](docs/features.md);
[AGENTS.md](AGENTS.md) is what to run before pushing and the traps already stepped in. In short:

```sh
cd backend  && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test
cd backend  && PGURL=postgres://moneta:moneta@localhost:5499/moneta tests/run-sql-tests.sh
cd frontend && npm run build && npm run check && npm run lint && npm test
```

The SQL tests and the database-backed Rust suites want a throwaway PostgreSQL 18 of their own,
never one holding real data. CI runs the same commands inside the Nix dev shell; on `main` it
also builds both images and deploys them.

## Status

Working: shared accounts and expenses with the per-user rendering, balances and their
reconciliation, periodic and automation rules with delivery deduplication, the analysis with its
value-date/accrual toggle, the changelog, API keys, login-proxy sign-in, and export and import.

Missing: an upload client feeding `deliveries.submit` from a real bank export (deliberately
someone else's program, since it is per bank and needs credentials), receipts in the UI, and
push notifications. [`TODO.md`](TODO.md) is the current list.

## License

MIT. See [LICENSE](LICENSE).
