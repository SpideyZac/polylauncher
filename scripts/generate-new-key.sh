#!/bin/bash
# Moves PolyLauncher.key and PolyLauncher.key.pub to old_keys/ with their timestamp and generates new keys using pnpm tauri signer generate -w PolyLauncher.key

# Create old_keys directory if it doesn't exist
mkdir -p old_keys

# Store date
current_date=$(date +%Y%m%d_%H%M%S)

# Move existing keys to old_keys with timestamp
if [ -f "PolyLauncher.key" ]; then
  mv "PolyLauncher.key" "old_keys/PolyLauncher.key.$current_date"
fi

if [ -f "PolyLauncher.key.pub" ]; then
  mv "PolyLauncher.key.pub" "old_keys/PolyLauncher.key.pub.$current_date"
fi

# Generate new keys
pnpm tauri signer generate -w PolyLauncher.key
