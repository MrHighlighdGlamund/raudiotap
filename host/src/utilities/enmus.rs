use std::{future::Future, pin::Pin, task::{Context, Poll}, time::Duration};

use tokio::pin;

pub enum GuiMessage {
        BufferUnderrun,
        TextError(String),
        Log(String),
        ServerError(String),
    }
pub enum ClientMessage {
    Start,
    Stop,
    Disconnect,
    Delay(u32),
    Ping(f32),
    UdpChunkSize(u64),

    }


