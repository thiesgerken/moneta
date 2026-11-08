FROM node:24-bookworm-slim@sha256:01d566e8d94d8c619591d78652babfccd3c6992a3dffa9ed5ab39e11cf6c70e9 AS build

ENV NODE_ENV=production
WORKDIR /app

COPY package.json package-lock.json .npmrc ./
# The build tools are all devDependencies, which NODE_ENV=production would leave out.
RUN npm ci --include=dev

COPY . ./

# The commit the bundle is built from, for the menu to name. Declared here rather than at the
# top: an ARG invalidates every layer below it, and this one changes on every commit, so above
# the install it would reinstall the world each time. `COPY . ./` already busts this layer
# whenever the frontend changes; what this adds is a rebuild for commits that did not touch it.
ARG GIT_SHA=
RUN npm run build

FROM node:24-bookworm-slim@sha256:01d566e8d94d8c619591d78652babfccd3c6992a3dffa9ed5ab39e11cf6c70e9

WORKDIR /app
COPY --from=build /app .

ENV HOST=0.0.0.0
ENV PORT=80
EXPOSE 80
CMD ["node","build"]
