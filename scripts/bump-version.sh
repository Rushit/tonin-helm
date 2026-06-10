#!/usr/bin/env bash
set -euo pipefail

# bump-version.sh — bump the crate version (source of truth: /VERSION),
# mirror it into Cargo.toml [package].version, refresh Cargo.lock,
# and create a clean commit.
#
# Usage: scripts/bump-version.sh <X.Y.Z>

if [[ $# -ne 1 ]]; then
    echo "error: expected exactly one argument (the new version)" >&2
    echo "usage: $0 <X.Y.Z>" >&2
    exit 1
fi

NEW="$1"

if [[ ! "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
    echo "error: '$NEW' is not a valid SemVer string" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "error: working tree has uncommitted changes; commit or stash them first" >&2
    git status --short >&2
    exit 1
fi

if [[ ! -f VERSION ]]; then
    echo "error: VERSION file not found" >&2
    exit 1
fi

CURRENT=$(tr -d '[:space:]' < VERSION)

CARGO_CURRENT=$(awk '
    /^\[package\]/ { in_section=1; next }
    /^\[/ && in_section { in_section=0 }
    in_section && /^version[[:space:]]*=/ {
        match($0, /"[^"]+"/)
        print substr($0, RSTART+1, RLENGTH-2)
        exit
    }
' Cargo.toml)

if [[ "$CARGO_CURRENT" != "$CURRENT" ]]; then
    echo "error: VERSION ($CURRENT) and Cargo.toml ($CARGO_CURRENT) disagree" >&2
    echo "       Resync by editing both to match, then re-run this script." >&2
    exit 1
fi

if [[ "$NEW" == "$CURRENT" ]]; then
    echo "error: new version equals current version ($CURRENT); nothing to do" >&2
    exit 1
fi

echo "Current version: $CURRENT"
echo "New version:     $NEW"

# Update VERSION
echo "$NEW" > VERSION

# Update Cargo.toml [package].version
awk -v new="$NEW" '
    BEGIN { in_section=0; replaced=0 }
    /^\[package\]/ { in_section=1; print; next }
    /^\[/ && in_section { in_section=0 }
    in_section && !replaced && /^version[[:space:]]*=/ {
        print "version = \"" new "\""
        replaced=1
        next
    }
    { print }
' Cargo.toml > Cargo.toml.bak && mv Cargo.toml.bak Cargo.toml

# Verify the rewrite
VERIFY=$(awk '
    /^\[package\]/ { in_section=1; next }
    /^\[/ && in_section { in_section=0 }
    in_section && /^version[[:space:]]*=/ {
        match($0, /"[^"]+"/)
        print substr($0, RSTART+1, RLENGTH-2)
        exit
    }
' Cargo.toml)

if [[ "$VERIFY" != "$NEW" ]]; then
    echo "error: Cargo.toml rewrite failed (still reads '$VERIFY')" >&2
    exit 1
fi

echo "Refreshing Cargo.lock via cargo check..."
cargo check

git add VERSION Cargo.toml Cargo.lock
git commit -m "chore: release v$NEW"

echo
echo "Bumped $CURRENT → $NEW, committed."
echo "Next: make release VERSION=$NEW to tag and push."
