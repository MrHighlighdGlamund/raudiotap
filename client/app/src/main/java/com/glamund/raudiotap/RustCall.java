package com.glamund.raudiotap;

public class RustCall {
    static {
        System.loadLibrary("main"); // Replace with your actual Rust library name
    }

    public static native void start_audio_service();
    public static native void stop_audio_service();
}

