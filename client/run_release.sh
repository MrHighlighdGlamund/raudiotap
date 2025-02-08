#!/bin/bash

set -e
# Build and install release version of the Android app

# Build Rust component for release
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/ build --release

# Build the Android app in release mode
./gradlew assembleRelease

# Optionally sign the APK (if not already handled in Gradle)
# ./gradlew bundleRelease or sign the APK manually
adb uninstall com.glamund.raudiotap
# Install the release APK on the device (modify path if needed)
adb install app/build/outputs/apk/release/app-release.apk

# Start the app (make sure package name and activity match your app)
# adb shell am start -n com.glamund.raudiotap/.MainActivity

# Run the app
adb shell am start -n com.glamund.raudiotap/.MainActivity
