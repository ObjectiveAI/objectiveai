# Builder (pnpm)
FROM node:20 AS builder
# Public, build-time-only config (NEXT_PUBLIC_* are inlined into the client
# bundle by Next.js at build). Server secrets are NOT passed here — they are
# injected at runtime by Cloud Run (see cloudbuild.yaml --set-secrets).
ARG NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ARG NODE_ENV
ENV NODE_ENV=${NODE_ENV}
ARG NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY
ENV NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY=${NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY}
ARG NEXT_PUBLIC_SONARLY_PROJECT_KEY
ENV NEXT_PUBLIC_SONARLY_PROJECT_KEY=${NEXT_PUBLIC_SONARLY_PROJECT_KEY}
ARG NEXT_PUBLIC_SONARLY_INGEST_POINT
ENV NEXT_PUBLIC_SONARLY_INGEST_POINT=${NEXT_PUBLIC_SONARLY_INGEST_POINT}

# Install pnpm via corepack
RUN corepack enable && corepack prepare pnpm@latest --activate

# Install workspace dependencies
WORKDIR /root
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml .npmrc ./
COPY objectiveai-sdk-js/package.json objectiveai-sdk-js/
COPY objectiveai-web/package.json objectiveai-web/
COPY objectiveai-viewer/package.json objectiveai-viewer/
RUN pnpm install --frozen-lockfile

# Build JS SDK (workspace resolves "objectiveai" locally)
COPY objectiveai-json-schema/ objectiveai-json-schema/
COPY objectiveai-sdk-rs-wasm-js/ objectiveai-sdk-rs-wasm-js/
COPY objectiveai-sdk-js/ objectiveai-sdk-js/
RUN pnpm run build:js

# Build web
COPY objectiveai-web/ objectiveai-web/
RUN pnpm run build:web

# Runner
FROM node:20-slim AS runner
# Server config + secrets (AUTH_*, NEXTAUTH_*, IP_RSA_PUBLIC_KEY, USER_IP_HEADER)
# are injected at runtime by Cloud Run via --set-env-vars / --set-secrets,
# NOT baked into the image.
WORKDIR /root
COPY --from=builder /root/objectiveai-web/.next .next
COPY --from=builder /root/objectiveai-web/public public
COPY --from=builder /root/node_modules node_modules
COPY --from=builder \
    /root/objectiveai-web/package.json \
    /root/objectiveai-web/next.config.ts \
    ./
CMD ["npx", "next", "start"]
