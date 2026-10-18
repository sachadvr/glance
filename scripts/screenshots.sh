#!/usr/bin/env bash
# Regenerates docs/screenshots/*.png from a real terminal: builds glance, a demo repository, then lets VHS
# (https://github.com/charmbracelet/vhs) drive glance and take the screenshots.
#
#   brew install vhs        # or: go install github.com/charmbracelet/vhs@latest (needs ttyd and ffmpeg)
#   scripts/screenshots.sh
set -euo pipefail

repository="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repository"

command -v vhs >/dev/null || { echo "vhs is needed: https://github.com/charmbracelet/vhs#installation" >&2; exit 1; }

cargo build --release --quiet

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
scripts/screenshots/demo-repo.sh "$scratch"

# A theme of its own and no saved choice from the user's config: every run draws the same colours.
mkdir -p "$scratch/config/glance"
printf 'Tokyo Night' > "$scratch/config/glance/theme"

export DEMO_REPOSITORY="$scratch/weather"
export XDG_CONFIG_HOME="$scratch/config"
export PATH="$repository/target/release:$PATH"
export GIT_CONFIG_GLOBAL=/dev/null

vhs scripts/screenshots/screens.tape
rm -f docs/screenshots/.recording.gif
ls -1 docs/screenshots/*.png
