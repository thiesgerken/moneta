# Operations

Running Moneta for real: the containers, the scheduled job, signing in through a login proxy,
moving data in and out, and the command line. The [README](../README.md) has the short version
and the configuration table.

## Containers

Both images, `backend` and `frontend`, are built with Nix rather than from Dockerfiles, with the
same pinned toolchain as the dev shell and CI, for x86_64-linux:

```sh
nix build .#backend-image  && ./result | docker load
nix build .#frontend-image && ./result | docker load
```

That loads them under the names [`docker-compose.yaml`](../docker-compose.yaml) uses, whose
registry is the owner's own and not public. `docker compose up -d` then runs the whole stack:
traefik on <http://localhost:8485> routing `/graphql` to the backend and everything else to the
frontend, PostgreSQL 18 with its data in `./volumes/postgres`, and the `periodic` sidecar below.
It publishes the backend on 8181 as well, so a local `npm run dev` talks to it without any
change; run one backend or the other, not both.

The compose file is a development stack. Its `MONETA__API__SECRET_KEY` is a demo value that
anyone can read here, so generate a real one (`openssl rand -base64 32`) for anything that holds
real data, and remove the `MONETA__RENDITIER__*` lines unless you have a renditier to point them
at.

**PostgreSQL 18 is a hard requirement**: ids default to `uuidv7()`, which does not exist before
it. Migrations run automatically before every subcommand except `schema`.

## Periodic rules need a scheduler

`moneta periodic --run` generates the expenses periodic rules are due for. It is idempotent, and
it is deliberately not an in-process timer, which would fire once per backend replica and hide
its failures in the server log. Run it on a schedule: a Kubernetes CronJob, a systemd timer, or
the compose stack's `periodic` service, whose restart policy is the loop.

## Signing in through a login proxy

With `MONETA__API__TRUSTED_HEADER_AUTHENTICATION=true`, a reverse proxy that does the login
(Authelia, oauth2-proxy, traefik's forward auth, ...) can hand over who is calling in a
`Remote-User` header carrying an existing moneta user name. Such a user needs no password, so an
imported user works straight away. Nobody is created for a name moneta does not know: that
request is simply signed out, with no fallback to a password login or to a session cookie the
browser still holds. The names on both sides therefore have to agree exactly, including case.

The header is believed only when the TCP connection comes from an address in
`MONETA__API__TRUSTED_PROXIES`: a network such as `10.42.0.0/16` (the pod network the proxy runs
in), or a single host written as `10.42.0.5/32`. A bare address without a prefix length is
refused at startup. `X-Forwarded-For` and `X-Real-IP` play no part in that decision. Switching
the header on with an empty list is a startup error. From anywhere else the header is ignored and
logged.

This is safe only when the backend is reachable **solely through that proxy**, and when the
proxy **removes a `Remote-User` the client sent** before setting its own. Anything that can open
a connection from inside the allowlist can sign in as anybody. An API key still wins over the
header, and the header wins over a session cookie. Signing out is the proxy's business: the
app's own logout cannot end a session it does not keep.

## Export and import

`moneta export` writes the whole database's user data to stdout as JSON: users, categories,
accounts, expenses with their transactions, balances, periodic and automation rules, seen
deliveries and the expense changelog. Credentials stay out: no password hashes, API keys or
renditier tokens. Sessions are cookies and were never in the database. Ids are kept, so the
changelog still points at what it describes.

`moneta import` reads such a file back, and only into a database that holds no user data yet
(the migrations having run is fine; a single user is not). Anything else is refused before the
first write: an expense books across several users' accounts, so there is no replacing one
user's data without breaking what they share with the others. The file is checked for
duplicate ids and dangling references first, and whatever the database rejects after that
rolls the whole import back. Imported users have no password; give each one with
`moneta user --update <name>`, and set renditier tokens again with `--renditier-token`.

The file holds everything personal there is, so it goes nowhere but where you redirect it:
both commands are CLI-only, nothing is served over HTTP, and `*-export.json` is gitignored.
The export complements a `pg_dump` rather than replacing it: it is readable and independent
of the schema's internals, but it leaves the credentials behind and restores only into an empty
database.

## CLI

`moneta` is one binary; `serve` is just one of its subcommands.

```sh
moneta serve                         # run the API
moneta user --add|--update|--remove <name> | --list
moneta user --renditier-token <name>   # paste the token on stdin; empty clears it
moneta account --list x              # the flags take a dummy value, see below
moneta account --sync <id1> <id2> [--invert x]
moneta account --unsync <id>
moneta periodic --list               # rules and how many expenses each is due for
moneta periodic --run                # generate the expenses that are due
moneta export > moneta-export.json   # all users' data as JSON, see above
moneta import < moneta-export.json   # only into a database without user data
moneta schema                        # the GraphQL schema as SDL, for frontend/schema.graphql
```

Note the odd argument shapes on `account`: `--list` and `--invert` are declared as value-taking
options rather than flags, so they need a throwaway argument (`moneta account --list x`). Worth
fixing, hasn't been.
