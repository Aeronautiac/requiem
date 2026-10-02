#!/bin/sh
set -eu

refuse() {
  echo "refusing tag: '$tag'" >&2
  exit 1
}

tag="${SSH_ORIGINAL_COMMAND:-}"
case "$tag" in sha-*) ;; *) refuse ;; esac
case "${tag#sha-}" in '' | *[!0-9a-f]*) refuse ;; esac

cd /opt/yagami
printf 'YAGAMI_TAG=%s\n' "$tag" >.env
docker compose pull
docker compose up -d
docker image prune -f
