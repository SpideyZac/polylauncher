#!/bin/bash
# Sets the environment variable TAURI_SIGNING_PRIVATE_KEY to the content of PolyLauncher.key
# Sets the environment variable TAURI_SIGNING_PRIVATE_KEY_PASSWORD to the content of pass
# After, runs pnpm tauri build
# Finally, unsets the environment variables

export TAURI_SIGNING_PRIVATE_KEY=$(cat PolyLauncher.key)
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$(cat pass)
pnpm tauri build
unset TAURI_SIGNING_PRIVATE_KEY
unset TAURI_SIGNING_PRIVATE_KEY_PASSWORD
