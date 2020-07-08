# Moneta

## TODO

### Frontend+Backend

- automation using templates and expense deliveries
- conversion of prepayments to booking periods
- Was ist mit Rückzahlungen?
- edit/create/delete expenses, balances, categories, accounts
- analysis
- push notifications
- front page: stream of recent events

### Frontend

- on big screens: expandable rows for expense table?

### Backend

- express sql queries using diesel; common functionality -> postgres functions

## Install

- `openssl rand -base64 32` to obtain secret key
- `openssl ecparam -genkey -name prime256v1 -out vapid.pem` to obtain private key for push
- `openssl ec -in vapid.pem -pubout -outform DER | tail -c 65 | base64 | tr '/+' '_-' | tr -d '\n'` to obtain base64 encoded public key for use in js
