use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Metadata wrapping the message body
#[derive(Debug, Serialize, Deserialize )]
pub struct Message {
    /// Sequence number
    #[serde(default)]
    pub seq: u64, 
    /// Content and type of the message
    #[serde(flatten)]
    pub data: MessageData,
} 

/// The message body 
#[derive(Debug, Serialize, Deserialize )]
#[serde(tag="msg_type", content="data")]
pub enum MessageData {
    // Player movement protocol 
    /// Client: First time join
    #[serde(rename="helo")]  
    Hello{username: String},   
    /// Client: Reconnect
    #[serde(rename="back")]  
    Back{token: Uuid},  
    /// Server: Accept player
    #[serde(rename="welc")]  
    Welcome{username: String, token: Uuid },  
    /// Server: Someone joined
    #[serde(rename="plrj")]  
    PlayerJoined{username: String},  
    /// Server: Someone left 
    #[serde(rename="plrl")]  
    PlayerLeft{username: String},  
    
    // Chat 
    //// Client: Say this in this chat 
    #[serde(rename="chas")]  
    ChatSend{chat_target: String, chat_content: String},   
    /// Server: This client said this in this chat
    #[serde(rename="chat")]  
    ChatSent{chat_sender: String, chat_target: String, chat_content: String},  
    /// Server+Client, this went wrong 
    #[serde(rename="err")]   
    Error{kind: String, info: String ,details: String},  


    /// Sync
    /// Server: This is how much happened before you joined
    #[serde(rename="rech")]  
    RecapHead{count: usize, chunk_sz: usize},
    /// Server: This is what happened before you joined
    #[serde(rename="recx")]  
    RecapTail{start: usize, msgs: Vec<Message>},  

    // Misc
    /// Server/Client: Echo 
    #[serde(rename="echo")] 
    Echo(serde_json::Value), 
} 


