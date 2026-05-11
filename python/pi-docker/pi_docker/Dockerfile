FROM node:24-bookworm-slim


ENV NPM_CONFIG_PREFIX=/opt/pi-docker/npm-global
ENV PATH=/opt/pi-docker/npm-global/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    bash \
    ca-certificates \
    curl \
    fd-find \
    git \
    less \
    openssh-client \
    ripgrep \
    socat \
    vim-tiny \
  && rm -rf /var/lib/apt/lists/*

ARG PI_VERSION=0.73.1
RUN npm install -g "@mariozechner/pi-coding-agent@${PI_VERSION}"

ARG PI_PACKAGES="@ollama/pi-web-search"
RUN if [ -n "${PI_PACKAGES}" ]; then npm install -g ${PI_PACKAGES}; fi

COPY docker-entrypoint.sh /usr/local/bin/pi-docker-entrypoint
RUN chmod +x /usr/local/bin/pi-docker-entrypoint

WORKDIR /workspace

ENTRYPOINT ["pi-docker-entrypoint"]
CMD []
