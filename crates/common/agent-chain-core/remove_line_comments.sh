#!/usr/bin/env bash
# remove_line_comments.sh
# Recursively remove lines whose first non-whitespace characters start with //,
# including /// and //!.

set -euo pipefail

find . -type f ! -path '*/.git/*' -print0 |
while IFS= read -r -d '' file; do
  # Delete lines that:
  #   - may start with spaces/tabs
  #   - then have "//"
  # This removes:
  #   // ...
  #   /// ...
  #   //! ...
  #   and any variant with leading whitespace.
  sed -i -E '/^[[:space:]]*\/\//d' "$file"
done
