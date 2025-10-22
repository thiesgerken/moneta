FROM node:24-bookworm-slim@sha256:8c996a5c6dbf6fe218caf6af9de865135991cb9d85bcd7116850908ae038d278 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:8c996a5c6dbf6fe218caf6af9de865135991cb9d85bcd7116850908ae038d278

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
