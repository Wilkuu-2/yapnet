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

use super::ChatSetup;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Metadata wrapping the message body
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Message {
    /// Sequence number
    #[serde(default)]
    pub seq: u64,
    /// Content and type of the message
    #[serde(flatten)]
    pub data: MessageData,
}

/// The message body
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "msg_type", content = "data")]
pub(crate) enum MessageData {
    /// Server: Game setup
    #[serde(rename = "stup")]
    Setup { chats: Vec<ChatSetup> },
    // Player movement protocol
    /// Client: First time join;
    #[serde(rename = "helo")]
    Hello { username: String },
    /// Client: Reconnect
    #[serde(rename = "back")]
    Back { token: Uuid },
    /// Server: Accept player
    #[serde(rename = "welc")]
    Welcome { username: String, token: Uuid },
    /// Server: Someone joined
    #[serde(rename = "plrj")]
    PlayerJoined { username: String },
    /// Server: Someone left
    #[serde(rename = "plrl")]
    PlayerLeft { username: String },

    // Chat
    //// Client: Say this in this chat
    #[serde(rename = "chas")]
    ChatSend {
        chat_target: String,
        chat_content: String,
    },
    /// Server: This client said this in this chat
    #[serde(rename = "chat")]
    ChatSent {
        chat_sender: String,
        chat_target: String,
        chat_content: String,
    },
    /// Server+Client, this went wrong
    #[serde(rename = "err")]
    Error {
        kind: String,
        info: String,
        details: String,
    },

    /// Sync
    /// Server: This is how much happened before you joined
    #[serde(rename = "rech")]
    RecapHead { count: usize, chunk_sz: usize },
    /// Server: This is what happened before you joined
    #[serde(rename = "recx")]
    RecapTail { start: usize, msgs: Vec<Value> },

    // Misc
    /// Server/Client: Echo
    #[serde(rename = "echo")]
    Echo(serde_json::Value),
}

impl MessageData {
    pub fn get_subject_player(&self) -> Option<String> {
        match self {
            Self::PlayerLeft { username } => Some(username.clone()),
            Self::PlayerJoined { username } => Some(username.clone()),
            Self::ChatSent { chat_sender, .. } => Some(chat_sender.clone()),
            _ => None,
        }
    }
    pub fn get_chat_name(&self) -> Option<String> {
        match self {
            Self::ChatSent { chat_target, .. } => Some(chat_target.clone()),
            Self::ChatSend { chat_target, .. } => Some(chat_target.clone()),
            _ => None,
        }
    }

    pub fn is_global(&self) -> bool {
        match self {
            Self::PlayerLeft { .. } | Self::PlayerJoined { .. } | Self::Setup { .. } => true,
            _ => false,
        }
    }
}

impl From<&Message> for String {
    fn from(val: &Message) -> Self {
        serde_json::to_string(val).expect("Right now we do not handle errors in serialization")
    }
}

impl From<MessageData> for Message {
    fn from(value: MessageData) -> Self {
        Message {
            seq: 0,
            data: value,
        }
    }
}

impl From<MessageData> for String {
    fn from(val: MessageData) -> Self {
        let msg: &Message = &val.into();
        msg.into()
    }
}
