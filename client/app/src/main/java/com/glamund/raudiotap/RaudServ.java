
package com.glamund.raudiotap;

import android.app.*;
import android.content.Context;
import android.content.Intent;
import android.os.*;
import android.util.Log;

public class RaudServ extends Service {

    private static final String CHANNEL_ID = "ForegroundServiceChannel";
    private static final int NOTIFICATION_ID = 1;

    @Override
    public void onCreate() {
        super.onCreate();
        createNotificationChannel();
    }

    private void createNotificationChannel() {
        // Create a NotificationChannel for the service
        NotificationChannel serviceChannel = new NotificationChannel(
                CHANNEL_ID,
                "Background Audio Service",
                NotificationManager.IMPORTANCE_DEFAULT  // Use IMPORTANCE_DEFAULT for visibility
        );
        NotificationManager manager = getSystemService(NotificationManager.class);
        if (manager != null) {
            manager.createNotificationChannel(serviceChannel);
        }
    }



    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && "STOP_SERVICE".equals(intent.getAction())) {
            // Stop the service if the stop action is triggered
            stopForeground(true); // Remove the foreground status
            stopSelf(); // Stop the service
            stopService();
            return START_NOT_STICKY; // Prevent the service from being restarted
        }

        Notification notification = createNotification();
        startForeground(NOTIFICATION_ID, notification); // Start the foreground service with the notification

        // Log to ensure onStartCommand is being called
        Log.d("RaudServ", "Foreground service started with notification");

        // Start Rust service
        startRustService();

        return START_NOT_STICKY;
    }
    private void startRustService() {
        // This calls your Rust function inside the .aar
        RustCall.start_audio_service();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
    private Notification createNotification() {
        // Intent to stop the service when the "Stop" button is clicked
        Intent stopIntent = new Intent(this, RaudServ.class);
        stopIntent.setAction("STOP_SERVICE");  // Custom action to stop the service
        // PendingIntent stopPendingIntent = PendingIntent.getService(this, 0, stopIntent, PendingIntent.FLAG_UPDATE_CURRENT);
        PendingIntent stopPendingIntent = PendingIntent.getService(
    this, 
    0, 
    stopIntent, 
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.S 
        ? PendingIntent.FLAG_IMMUTABLE 
        : PendingIntent.FLAG_UPDATE_CURRENT
);

        Notification.Builder builder;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            builder = new Notification.Builder(this, CHANNEL_ID)
                    .setContentTitle("Playing Audio")
                    .setContentText("Foreground Service Running")
                    .setSmallIcon(android.R.drawable.ic_media_play)
                    .setOngoing(true)  // Make it clear that th service is ongoing
                    .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Stop", stopPendingIntent); // Stop button action
        } else {
            builder = new Notification.Builder(this)
                    .setContentTitle("Playing Audio")
                    .setContentText("Foreground Service Running")
                    .setSmallIcon(android.R.drawable.ic_media_play)
                    .setOngoing(true)
                    .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Stop", stopPendingIntent); // Stop button action
        }
        return builder.build();
    }

    public static void stopService() {

        RustCall.stop_audio_service();
    }
    public void onDestroy() {
        RustCall.stop_audio_service();
        super.onDestroy();
        Log.d("RaudServ", "Service is being destroyed.");
    }
}

