pub enum GuiMessage {
    Start,
    Stop,
    Delay(f32),
}
pub enum ServerMessage {
    Log(String),
    Connected,
    Disconnected,
    Started,
    Stopped,
    Delay(f32),
}
pub enum ClientSate {
    Running,
    Disconnected,
    Connected,
}
