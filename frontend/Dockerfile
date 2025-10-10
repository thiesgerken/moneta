FROM node:24-bookworm-slim@sha256:c0eadb278e6b0b608681b359291592692979133567710eaa703d6f989f33c6c1 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:c0eadb278e6b0b608681b359291592692979133567710eaa703d6f989f33c6c1

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
