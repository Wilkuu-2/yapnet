// Copyright 2024 Jakub Stachurski
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{ChatSetup, MessageDataV2, MessageV2};
yapnet_macro::protocol_body! {
    #[derive(yapnet_macro::MessageDataV2, Serialize, Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type="helo")]
    pub struct TestMessage {
        pub username: String,
    }
    /// Server: Game setup
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type="setp")]
    pub struct Setup { pub chats: Vec<ChatSetup>, }
    // Player movement protocol
    /// Client: First time join
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(msg_type = "helo", global = true)]
    pub struct Hello { pub username: String }
    /// Client: Reconnect
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "back")]
    pub struct Back { pub token: Uuid }
    /// Server: Accept player
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "welc")]
    pub struct Welcome { pub username: String, pub token: Uuid }
    /// Server: Someone joined
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "plrj")]
    pub struct PlayerJoined { pub username: String }
    /// Server: Someone left
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "plrl")]
    pub struct PlayerLeft { pub username: String }

    // Chat
    //// Client: Say this in this chat
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "chas")]
    pub struct ChatSend {
        pub chat_target: String,
        pub chat_content: String,
    }
    /// Server: This client said this in this chat
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "chat")]
    pub struct ChatSent {
        pub chat_sender: String,
        pub chat_target: String,
        pub chat_content: String,
    }
    /// Server+Client, this went wrong
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "err")]
    pub struct Error {
        pub kind: String,
        pub info: String,
        pub details: String,
    }
    /// Sync
    /// Server: This is how much happened before you joined
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "rech")]
    pub struct RecapHead { pub count: usize, pub chunk_sz: usize }
    /// Server: This is what happened before you joined
    #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "recx")]
    pub struct RecapTail { pub start: usize, pub msgs: Vec<Value> }

    // Misc
    /// Server/Client: Echo
        #[derive(yapnet_macro::MessageDataV2, Serialize,Deserialize, Debug, Clone)]
    #[msg_data(global=true, msg_type = "echo")]
    pub struct Echo(serde_json::Value);
}

impl Into<String> for &MessageV2 {
    fn into(self) -> String {
        serde_json::to_string(self).expect("Right now we do not handle errors in serialization")
    }
}

impl From<MessageV2Enum> for MessageV2 {
    fn from(value: MessageV2Enum) -> Self {
        MessageV2 {
            seq: 0,
            data: value,
        }
    }
}

impl Into<String> for MessageV2Enum {
    fn into(self) -> String {
        let msg: &MessageV2 = &self.into();
        return msg.into();
    }
}

pub trait IntoMessage {
    fn into_message(self) -> MessageV2;
    fn into_numbered_message(self, seq: u64) -> MessageV2 
    where
        Self: Sized
    {
        let mut msg = self.into_message();
        msg.seq = seq;
        msg
    }
}

impl<T: Into<MessageV2Enum>> IntoMessage for T {
    fn into_message(self) -> MessageV2 {
        let enem: MessageV2Enum = self.into(); 
        enem.into()
        
    }

}
