pub enum GuiMessage {
        BufferUnderrun,
        TextError(String),
        Log(String),
        ServerError(String),
    }
