FROM node:22-bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends dumb-init \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY package*.json ./

ENV PUPPETEER_SKIP_DOWNLOAD=true

# Install all dependencies (including dev dependencies for build)
RUN npm ci

# Install Chrome for Testing AND the system dependencies
RUN npx --yes puppeteer browsers install chrome --install-deps

# Copy source code and build
COPY . .
RUN npm run build

# Remove dev dependencies after build
RUN npm ci --omit=dev && npm cache clean --force

ENTRYPOINT ["dumb-init","--"]
CMD ["node","dist/server.js"]