FROM node:22-bookworm

# Chrome runtime dependencies (Debian 12)
RUN apt-get update && apt-get install -y --no-install-recommends \
    dumb-init ca-certificates wget xdg-utils fonts-liberation \
    libasound2 libatk-bridge2.0-0 libatk1.0-0 libatspi2.0-0 \
    libcups2 libdbus-1-3 libdrm2 libgbm1 libgtk-3-0 libnspr4 \
    libnss3 libu2f-udev libvulkan1 libx11-xcb1 libxcomposite1 \
    libxdamage1 libxfixes3 libxkbcommon0 libxrandr2 libxshmfence1 \
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
