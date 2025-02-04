#!/bin/bash
# declare STRING variable
# print variable on a screen
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build
./gradlew build
./gradlew installDebug
adb shell am start -n com.glamund.raudiotap/.MainActivity
