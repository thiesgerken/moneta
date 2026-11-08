FROM node:26-bookworm-slim@sha256:add5c17c14752c54d2f3d519ec46a3815684d41c33f198bcb09fb998f7dcbf68 AS build

ENV NODE_ENV=production
WORKDIR /app
RUN npm i -g pnpm@12.18.0

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

FROM node:26-bookworm-slim@sha256:add5c17c14752c54d2f3d519ec46a3815684d41c33f198bcb09fb998f7dcbf68

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
