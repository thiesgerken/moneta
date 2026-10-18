# Moneta documentation

Personal finance tracking for a small number of people who share accounts and expenses. Rust
backend serving GraphQL over PostgreSQL, SvelteKit frontend.

## Reading order

Coming to this repo cold, in this order:

1. **[concepts.md](concepts.md)** — the data model. Two layers of tables, three dates on an
   expense, and what "my share" means. Nothing else in the backend makes sense before this
   one, and it is short.
2. **[features.md](features.md)** — what the application actually does, screen by screen.
3. **[backend.md](backend.md)** or **[frontend.md](frontend.md)** — code layout and the
   conventions of whichever half you are changing.

Two more, for specific work:

- **[analysis-dashboard.md](analysis-dashboard.md)** — the `/analysis` section in detail: what
  each of its seven tabs shows, the queries behind them, and the decisions that shaped both.
- **[../frontend/DESIGN.md](../frontend/DESIGN.md)** — what the UI is supposed to look like.
  Read it before adding a screen.

## The rest of the repo

| File | What it is |
| --- | --- |
| [`../README.md`](../README.md) | running it locally, configuration, the CLI |
| [`../AGENTS.md`](../AGENTS.md) | how to work here: what to run to verify, and the traps already stepped in |
| [`../TODO.md`](../TODO.md) | what is deliberately not built yet |
| `../frontend/schema.graphql` | the API reference, generated and documented in place |

`AGENTS.md` and this folder do different jobs and both are worth having. These documents
explain *what the system is*; `AGENTS.md` explains *what goes wrong when you change it* — the
deadlock a nested dataloader causes, the cache a new mutation has to invalidate, the 0,00 €
balances that look like noise and are load-bearing. Neither replaces the other.
