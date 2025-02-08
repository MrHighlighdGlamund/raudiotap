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
    Delay(f32),
    Ping(f32),
    UdpChunkSize(u64),

}
