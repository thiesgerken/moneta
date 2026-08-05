#!/bin/sh
# Applies the migrations to an empty database and runs the SQL tests against it.
#
# The migrations are applied with psql rather than diesel_cli because diesel_cli links libpq,
# which the rest of the build does not need. They are named so that lexical order is apply
# order.
set -eu

PGURL="${PGURL:-postgres://moneta:moneta@localhost:5433/moneta}"
cd "$(dirname "$0")/.."

for migration in migrations/*/up.sql; do
    echo "== applying $migration"
    psql -v ON_ERROR_STOP=1 -q "$PGURL" -f "$migration"
done

# each test file wraps itself in BEGIN/ROLLBACK and raises on a failed assertion, so
# ON_ERROR_STOP is what turns a wrong view into a non-zero exit
for test in tests/sql/*.sql; do
    echo "== running $test"
    psql -v ON_ERROR_STOP=1 -q "$PGURL" -f "$test"
done

echo "== all sql tests passed"
