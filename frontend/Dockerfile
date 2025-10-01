FROM node:24-bookworm-slim@sha256:9cd5e2bb1f5c5a6636e2946b3ea2d54a4148989537f8cfdfbb43c88c79dd1d5b AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:9cd5e2bb1f5c5a6636e2946b3ea2d54a4148989537f8cfdfbb43c88c79dd1d5b

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
