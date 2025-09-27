FROM node:24-bookworm-slim@sha256:0cce74a5708f603925e2bf01929da8d71e92b5e2493fcfb662d5a8ffed2d8ef1 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:0cce74a5708f603925e2bf01929da8d71e92b5e2493fcfb662d5a8ffed2d8ef1

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
