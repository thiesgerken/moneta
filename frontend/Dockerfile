FROM node:24-bookworm-slim@sha256:04d9cbb7297edb843581b9bb9bbed6d7efb459447d5b6ade8d8ef988e6737804 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:04d9cbb7297edb843581b9bb9bbed6d7efb459447d5b6ade8d8ef988e6737804

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
