FROM node:24-bookworm-slim@sha256:61bf992754b4ab288d41cb92c25392195d0035871b4723a0abd30a49dcba356c AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:61bf992754b4ab288d41cb92c25392195d0035871b4723a0abd30a49dcba356c

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
