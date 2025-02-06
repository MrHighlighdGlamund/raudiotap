pub enum GuiMessage {
    Start,
    Stop,
}
pub enum ServerMessage {
    Log(String),
    Connected,
    Disconnected,
}
