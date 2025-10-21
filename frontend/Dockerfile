FROM node:24-bookworm-slim@sha256:3207eadbf6e1cb68087bcde8d87ab52e904830dce49af6382bdda015bbc5a423 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:3207eadbf6e1cb68087bcde8d87ab52e904830dce49af6382bdda015bbc5a423

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
