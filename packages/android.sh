#! /bin/bash -e

export ANDROID_NDK="${HOME}/Android/Sdk/ndk/29.0.14206865"

cargo build --example hnefatafl-client --target aarch64-linux-android --no-default-features