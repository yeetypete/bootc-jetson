#!/usr/bin/env bash

# Download the newest CI disk image built from a branch, by default the current one.

set -euo pipefail

VARIANT="${1:?Usage: download.sh <variant> [branch]}"
BRANCH="${2:-$(git branch --show-current)}"
DISK_NAME="bootc-jetson-${VARIANT}"

read -r run url < <(gh api "repos/{owner}/{repo}/actions/artifacts?name=disk-image-${VARIANT}&per_page=100" \
    --jq '[.artifacts[] | select(.workflow_run.head_branch == "'"$BRANCH"'" and (.expired | not))]
        | sort_by(.created_at) | last | "\(.workflow_run.id) \(.archive_download_url)"')
if [[ $url == null ]]; then
    echo "No disk-image-${VARIANT} artifact found for ${BRANCH}." >&2
    exit 1
fi

echo "Downloading the disk image from run ${run}."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# GitHub redirects to a short-lived URL, which aria2c downloads in parallel.
aria2c -x 16 -s 16 --console-log-level=warn --summary-interval=0 --download-result=hide -d "$tmp" -o artifact.zip \
    "$(curl -fsS -o /dev/null -w '%{redirect_url}' -H "Authorization: Bearer $(gh auth token)" "$url")"
unzip -p "$tmp/artifact.zip" "${DISK_NAME}.img.zst" | zstd -d -f -o "${DISK_NAME}.img"
