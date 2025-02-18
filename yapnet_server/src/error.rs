

#[derive(Debug, Clone)]
pub enum ClientError {
    NameTaken(String), 
    InvalidToken,
    NoLogin, 
    NoPermission,
    InvalidObject(UserId), 
    InvalidSubject(UserId), 
    InvalidChat(ChatId), 
    InvalidAction(String),
    Custom(String),
}

impl IntoMessage for ClientError {
    fn into_message(self) -> Message {
        let e: MessageV2Enum = MessageV2Enum::BodyError(match self {
    }
}

impl std::fmt::Display for ClientError {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       write!(f, "{:?}", self)
   } 
}
impl std::error::Error for ClientError {}

#[derive(Debug, Clone)]
pub enum ServerError {
    Custom(String),
}

impl std::fmt::Display for ServerError {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       write!(f, "{:?}", self)
   } 
}
impl std::error::Error for ServerError {}
