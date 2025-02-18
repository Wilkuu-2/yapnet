use crate::{prelude::{IntoMessage, Message, MessageV2Enum, YnError}, protocol::{ChatId, UserId}};


#[derive(Debug, Clone)]
pub enum ClientError {
    NameTaken(String), 
    InvalidToken,
    NoLogin, 
    NoPermission(String, String),
    InvalidObject(UserId, String), 
    InvalidSubject(UserId, String), 
    InvalidChat(ChatId, String), 
    InvalidAction(String, String),
    Custom(String, String),
}

impl From<ClientError> for YnError {
    fn from(value: ClientError) -> Self {
        match value {
            ClientError::NameTaken(name) 
            => Self::new("NameTaken", &format!("The name: {} is taken", name), 
                format!("{{ \"invalid_name\": \"{}\" }}",name)),
            ClientError::InvalidToken
            => Self::new("InvalidToken", "The token you gave is invalid", ""),
            ClientError::NoLogin 
            => Self::new("NoLogin", "The action requires login", ""),
            ClientError::NoPermission(object, reason)
            => Self::new("NoPermission", &format!("The action on {} requires permissions you don't have", object), simple_json_object("reason", reason)),
            ClientError::InvalidObject(id, reason)  
            => Self::new("InvalidObject", &format!("{}, cannot be the object of that action", id), simple_json_object("reason", reason)),
            ClientError::InvalidSubject(id, reason)
            => Self::new("InvalidSubject", &format!("{}, cannot be the subject of that action", id), simple_json_object("reason", reason)),
            ClientError::InvalidChat(id, reason)
            => Self::new("InvalidChat", &format!("{}, cannot be targeted for that action", id), simple_json_object("reason", reason)),
            ClientError::InvalidAction(id, reason)
            => Self::new("InvalidAction", &format!("the action, {}, cannot be performed", id), simple_json_object("reason", reason)),
            ClientError::Custom(info, details)
            => Self::new("Custom", info, details),
        }
    }
} 

fn simple_json_object<F,V>(field: F, value: V) -> String
where 
F: Into<String>, 
V: Into<String>, 
{
    format!("{{ \"{}\":\"{}\" }}", field.into(), value.into())
}

impl IntoMessage for ClientError {
    fn into_message(self) -> Message {
        let e = MessageV2Enum::BodyYnError(self.into()); 
        e.into_message()
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
    InvalidToken,
    AlreadyJoinedOrLeft, 
    NameTaken(String), 
    Custom(String, String),
}

impl From<ServerError> for YnError {
    fn from(value: ServerError) -> Self {
        match value {
            ServerError::InvalidToken
            => Self::new("InvalidToken", "The token you gave is invalid", ""),
            ServerError::AlreadyJoinedOrLeft
            => Self::new("InvalidChat", "The user already joined, or left" ,""),
            ServerError::NameTaken(name) 
            => Self::new("NameTaken", format!("The name: {} is taken", name), 
                format!("{{ \"invalid_name\": \"{}\" }}",name)),
            ServerError::Custom(info, details)
            => Self::new("ServerError", info, details),
        }
    }
} 

impl IntoMessage for ServerError {
    fn into_message(self) -> Message {
        MessageV2Enum::BodyYnError(self.into()).into_message()
    }
}

impl std::fmt::Display for ServerError {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       write!(f, "{:?}", self)
   } 
}

impl std::error::Error for ServerError {}
