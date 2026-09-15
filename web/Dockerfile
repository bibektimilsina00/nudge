# The marketing site.
FROM node:24-slim AS build
RUN corepack enable
WORKDIR /srv

# pnpm-workspace.yaml as well as the lockfile: it carries `allowBuilds`, and
# without it here pnpm refuses the install over a script it has not been told
# about -- a failure that only happens in a clean checkout, which is to say only
# in the place it matters.
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

COPY . .
# Baked in at build time, not read at runtime: NEXT_PUBLIC_* is substituted into
# the bundle during the build, so setting it in the container does nothing.
ARG NEXT_PUBLIC_API=/
ENV NEXT_PUBLIC_API=${NEXT_PUBLIC_API}
RUN pnpm build

FROM node:24-slim AS run
WORKDIR /srv
ENV NODE_ENV=production
# Next's standalone output, which is the server and only the dependencies it
# actually reached -- a fraction of node_modules.
COPY --from=build /srv/.next/standalone ./
COPY --from=build /srv/.next/static ./.next/static
COPY --from=build /srv/public ./public
EXPOSE 3000
CMD ["node", "server.js"]
