#!/usr/bin/env bash
#
# Download prebuilt PostgreSQL binaries for embedding in euro-server.
#
# Usage:
#   ./scripts/download-postgres.sh <os> <arch> <dest_dir>
#
# Arguments:
#   os       - darwin | linux | windows
#   arch     - aarch64 | x86_64
#   dest_dir - directory to place postgres binaries (e.g. crates/app/euro-server/postgres)
#
# This script downloads a minimal PostgreSQL distribution containing only:
#   bin/initdb, bin/pg_ctl, bin/postgres, bin/createdb
#   lib/ (required shared libraries)
#   share/postgresql/ (timezone and locale data needed by initdb)

set -euo pipefail

PG_MAJOR="16"
PG_VERSION="16.8-1"

OS="${1:?Usage: $0 <os> <arch> <dest_dir>}"
ARCH="${2:?Usage: $0 <os> <arch> <dest_dir>}"
DEST="${3:?Usage: $0 <os> <arch> <dest_dir>}"

DEST="$(cd "$(dirname "$DEST")" && pwd)/$(basename "$DEST")"

mkdir -p "$DEST"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading PostgreSQL ${PG_VERSION} for ${OS}-${ARCH}..."

case "${OS}-${ARCH}" in
  darwin-aarch64)
    URL="https://get.enterprisedb.com/postgresql/postgresql-${PG_VERSION}-osx-binaries.zip"
    curl -fSL "$URL" -o "$TMPDIR/pg.zip"
    unzip -q "$TMPDIR/pg.zip" -d "$TMPDIR/extract"
    PG_ROOT="$TMPDIR/extract/pgsql"
    ;;
  darwin-x86_64)
    URL="https://get.enterprisedb.com/postgresql/postgresql-${PG_VERSION}-osx-binaries.zip"
    curl -fSL "$URL" -o "$TMPDIR/pg.zip"
    unzip -q "$TMPDIR/pg.zip" -d "$TMPDIR/extract"
    PG_ROOT="$TMPDIR/extract/pgsql"
    ;;
  linux-x86_64)
    URL="https://get.enterprisedb.com/postgresql/postgresql-${PG_VERSION}-linux-x64-binaries.tar.gz"
    curl -fSL "$URL" -o "$TMPDIR/pg.tar.gz"
    mkdir -p "$TMPDIR/extract"
    tar xzf "$TMPDIR/pg.tar.gz" -C "$TMPDIR/extract"
    PG_ROOT="$TMPDIR/extract/pgsql"
    ;;
  linux-aarch64)
    URL="https://get.enterprisedb.com/postgresql/postgresql-${PG_VERSION}-linux-arm64-binaries.tar.gz"
    curl -fSL "$URL" -o "$TMPDIR/pg.tar.gz"
    mkdir -p "$TMPDIR/extract"
    tar xzf "$TMPDIR/pg.tar.gz" -C "$TMPDIR/extract"
    PG_ROOT="$TMPDIR/extract/pgsql"
    ;;
  windows-x86_64)
    URL="https://get.enterprisedb.com/postgresql/postgresql-${PG_VERSION}-windows-x64-binaries.zip"
    curl -fSL "$URL" -o "$TMPDIR/pg.zip"
    unzip -q "$TMPDIR/pg.zip" -d "$TMPDIR/extract"
    PG_ROOT="$TMPDIR/extract/pgsql"
    ;;
  *)
    echo "ERROR: Unsupported platform: ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

if [ ! -d "$PG_ROOT" ]; then
  echo "ERROR: PostgreSQL root not found after extraction" >&2
  exit 1
fi

echo "Extracting required binaries..."

# Copy binaries
mkdir -p "$DEST/bin"
for bin in initdb pg_ctl postgres createdb; do
  if [ -f "$PG_ROOT/bin/$bin" ]; then
    cp "$PG_ROOT/bin/$bin" "$DEST/bin/"
  elif [ -f "$PG_ROOT/bin/$bin.exe" ]; then
    cp "$PG_ROOT/bin/$bin.exe" "$DEST/bin/"
  else
    echo "WARNING: $bin not found in $PG_ROOT/bin/" >&2
  fi
done

# Copy shared libraries (needed at runtime)
if [ -d "$PG_ROOT/lib" ]; then
  mkdir -p "$DEST/lib"
  # Copy all .so, .dylib, .dll files
  find "$PG_ROOT/lib" -maxdepth 1 \( -name "*.so*" -o -name "*.dylib" -o -name "*.dll" \) \
    -exec cp {} "$DEST/lib/" \;
fi

# Copy share data (timezone, locale data needed by initdb)
if [ -d "$PG_ROOT/share/postgresql" ]; then
  mkdir -p "$DEST/share"
  cp -r "$PG_ROOT/share/postgresql" "$DEST/share/"
elif [ -d "$PG_ROOT/share" ]; then
  mkdir -p "$DEST/share"
  cp -r "$PG_ROOT/share" "$DEST/share/"
fi

# Make binaries executable
chmod +x "$DEST/bin/"* 2>/dev/null || true

# Print summary
echo ""
echo "PostgreSQL ${PG_VERSION} binaries installed to: $DEST"
echo "Contents:"
find "$DEST" -type f | head -30
TOTAL_SIZE=$(du -sh "$DEST" | cut -f1)
echo "Total size: $TOTAL_SIZE"
