#!/bin/bash

export TAURI_SIGNING_PRIVATE_KEY=$(cat PolyLauncher.key)
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$(cat pass)

# Generate new keys
./scripts/generate-new-key.sh

pnpm tauri build
unset TAURI_SIGNING_PRIVATE_KEY
unset TAURI_SIGNING_PRIVATE_KEY_PASSWORD
