#!/bin/bash

# Remove specified directories from root and all subdirectories
find . -type d \( -name ".svelte-kit" -o -name ".turbo" -o -name "build" -o -name "dist" -o -name "node_modules" \) -prune -exec rm -rf {} +
