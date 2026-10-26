#!/bin/bash
# Builds a signed RemuxDV release APK.
# Usage: scripts/build-release.sh <version>, e.g. scripts/build-release.sh 0.20.0-remuxdv.1
# Signing settings are read from ~/.config/remuxdv/signing.properties (or $REMUXDV_SIGNING), which
# holds keystore.file, keystore.password, signing.key.alias and signing.key.password.
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: $0 <version>}"
signing="${REMUXDV_SIGNING:-$HOME/.config/remuxdv/signing.properties}"

args=()
while IFS= read -r line; do
	[[ $line == *=* ]] && args+=("-P$line")
done < "$signing"

./gradlew --no-daemon :app:assembleRelease -Pjellyfin.version="$version" "${args[@]}"
ls app/build/outputs/apk/release/*.apk
