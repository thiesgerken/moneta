FROM node:24-bookworm-slim@sha256:3638d9a6fe4030bd716be989438248074489337ba3275657f93595428be4fc03 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm@12.8.1

# pnpm-workspace.yaml carries allowBuilds; without it pnpm 10 refuses to install at all
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml .npmrc ./
RUN pnpm i --frozen-lockfile

COPY . ./

# The commit the bundle is built from, for the menu to name. Declared here rather than at the
# top: an ARG invalidates every layer below it, and this one changes on every commit, so above
# the install it would reinstall the world each time. `COPY . ./` already busts this layer
# whenever the frontend changes; what this adds is a rebuild for commits that did not touch it.
ARG GIT_SHA=
RUN pnpm build

FROM node:24-bookworm-slim@sha256:3638d9a6fe4030bd716be989438248074489337ba3275657f93595428be4fc03

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
