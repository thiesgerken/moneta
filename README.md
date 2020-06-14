# Moneta

## TODO

### Frontend+Backend

- automation using templates and expense deliveries
- conversion of prepayments to booking periods
- Was ist mit Rückzahlungen?
- expense and balances tables: sorted subqueries instead of fetching all data
- edit/create/delete expenses, balances, categories, accounts
- analysis

### Frontend

- Accounts and Categories should be loaded at app start (and invalidated on change)
- tables with expenses, balances, categories, accounts
- tables for cats and accounts: filter by owner id first
- augment received expenses with "meta" or "computed" or so field with information about
  - total expense cost
  - booking time end
- include colors of accounts and categories in the table cells

### Backend

- express sql queries using diesel; common functionality -> postgres functions

## Install

- `openssl rand -base64 32` to obtain secret key
- `openssl ecparam -genkey -name prime256v1 -out vapid.pem` to obtain private key for push
- `openssl ec -in vapid.pem -pubout -outform DER | tail -c 65 | base64 | tr '/+' '_-' | tr -d '\n'` to obtain base64 encoded public key for use in js
