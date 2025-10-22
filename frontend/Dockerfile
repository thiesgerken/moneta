FROM node:24-bookworm-slim@sha256:ee8cab03c55fe51ea1a4250a7bc037352835caae6a6926be4189837ffcbb307b AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:ee8cab03c55fe51ea1a4250a7bc037352835caae6a6926be4189837ffcbb307b

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
