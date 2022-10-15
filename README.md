# Moneta

Tool to manage personal finances.

## Install

- `openssl rand -base64 32` to obtain secret key
- `openssl ecparam -genkey -name prime256v1 -out vapid.pem` to obtain private key for push
- `openssl ec -in vapid.pem -pubout -outform DER | tail -c 65 | base64 | tr '/+' '_-' | tr -d '\n'` to obtain base64 encoded public key for use in js

## Use

- `moneta serve` starts a web server on port 8484 (needs a running mongo), same happens when using `docker-compose up -d`
- API is served on `/api` and documentation at `/api/doc`
