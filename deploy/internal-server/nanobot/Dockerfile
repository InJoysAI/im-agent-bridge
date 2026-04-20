FROM python:3.11-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    gnupg \
    ca-certificates \
    && curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash nanobot

RUN pip install --no-cache-dir "nanobot-ai[api]"

USER nanobot
WORKDIR /home/nanobot

EXPOSE 8900

CMD ["nanobot", "serve"]
