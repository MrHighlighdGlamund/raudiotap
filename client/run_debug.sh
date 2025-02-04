#!/bin/bash
# declare STRING variable
# print variable on a screen
set -e
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build
./gradlew build
adb uninstall com.glamund.raudiotap
./gradlew installDebug
adb shell am start -n com.glamund.raudiotap/.MainActivity
