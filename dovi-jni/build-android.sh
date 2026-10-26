#!/bin/bash
# Builds libdovi_jni.so for the app's ABIs into the exoplayer module's jniLibs.
# Needs rustup with the Android targets, cargo-ndk and the Android NDK.
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PATH"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$(ls -d "$HOME"/Android/Sdk/ndk/* | sort -V | tail -1)}"
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86_64 -P 23 \
	-o ../playback/media3/exoplayer/src/main/jniLibs build --release
