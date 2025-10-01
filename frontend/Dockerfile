FROM node:24-bookworm-slim@sha256:62609e66054203c83bc10f262f50d1affd898b7bcf44e0766c530f74432c3a6c AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm

COPY package.json pnpm-lock.yaml ./
RUN pnpm i

COPY . ./
RUN pnpm build

FROM node:24-bookworm-slim@sha256:62609e66054203c83bc10f262f50d1affd898b7bcf44e0766c530f74432c3a6c

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
