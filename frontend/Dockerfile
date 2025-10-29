FROM node:24-bookworm-slim@sha256:07a2fca567b37b38fe11eb6a38d0a6b33262d76607970ff76affc4b7a2581558 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:07a2fca567b37b38fe11eb6a38d0a6b33262d76607970ff76affc4b7a2581558

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
