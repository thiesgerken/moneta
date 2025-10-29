FROM node:24-bookworm-slim@sha256:0f2d677a7152ee7ac390837bd4fc36aca12f595411df5d4209f972660e34a7b6 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:0f2d677a7152ee7ac390837bd4fc36aca12f595411df5d4209f972660e34a7b6

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
