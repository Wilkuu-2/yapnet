use chrono::{DateTime, Utc};
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
use serde_json::Value;
use uuid::Uuid;
use yapnet_macro::MessageDataV2;

use super::{ChatId, ChatSetup, MessageV2, RoleId, UserId};
yapnet_macro::protocol_body! {
    /// Server: Game setup
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type="setp")]
    pub struct Setup { pub chats: Vec<ChatSetup>, }
    // Player movement protocol
    /// Client: First time join
    #[derive(MessageDataV2)]
    #[msg_data(msg_type = "helo", global = true)]
    pub struct Hello {
        #[msg_info(subject)]
        pub username: String
    }
    /// Client: Reconnect
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "back")]
    pub struct Back {
        pub token: Uuid
    }
    /// Server: Accept player
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "welc")]
    pub struct Welcome {
        #[msg_info(object)]
        pub username: String,
        pub token: Uuid
    }
    /// Server: Someone joined
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "plrj")]
    pub struct PlayerJoined {
        #[msg_info(subject)]
        pub username: String
    }
    /// Server: Someone left
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "plrl")]
    pub struct PlayerLeft {
        #[msg_info(subject)]
        pub username: String
    }

    // Chat
    /// Client: Say this in this chat
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "chas")]
    pub struct ChatSend {
        pub chat_target: String,
        pub chat_content: String,
    }
    /// Server: This client said this in this chat
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "chat")]
    pub struct ChatSent {
        #[msg_info(subject)]
        pub chat_sender: String,
        #[msg_info(chat)]
        pub chat_target: String,
        pub chat_content: String,
    }


    /// Server: This player has this role
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "revr")]
    pub struct RoleReveal {
        user: UserId,
        role: RoleId,
    }

    /// Server: This player has this role
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "revk")]
    pub struct KillReveal {
        user: UserId,
        role: RoleId,
    }

    /// Player actions
    /// Server: This are the actions you can do.
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "aavl")]
    pub struct AvailableActions {
        actions: Vec<String>,
    }


    /// Client: I do the following action with the following arguments
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "asub")]
    pub struct SubmitAction {
        action_id: String,
        args: Vec<String>,
    }

    /// Server: A vote has started in this room with the following theme and end time.
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "vstt")]
    pub struct VoteStart {
        #[msg_info(chat)]
        chat_id: ChatId,
        subject: String,
        end_time: DateTime<Utc>,
    }

    /// Client: I vote on this person in this chat
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "vsub")]
    pub struct SubmitVote {
        #[msg_info(object)]
        player_id: UserId,
        #[msg_info(chat)]
        chat_id: ChatId,
    }

    /// Server: This user votes on this person in this chat
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "vrsb")]
    pub struct VoteSubmission {
        #[msg_info(subject)]
        voter: UserId, // TODO: Make option
        #[msg_info(object)]
        target: UserId,
        #[msg_info(chat)]
        chat_id: ChatId,
    }

    /// Server: The vote ended with the following result
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "vres")]
    pub struct VoteResult {
        target: UserId,
        chat_id: ChatId,
    }

    /// Server: The action is submitted with the following result
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "ares")]
    pub struct ActionResult {
        success: bool,
        reason: String,
    }


    /// Server+Client, this went wrong
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "err")]
    pub struct Error {
        pub kind: String,
        pub info: String,
        pub details: String,
    }
    /// Sync
    /// Server: This is how much happened before you joined
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "rech")]
    pub struct RecapHead {
        pub count: usize,
        pub chunk_sz: usize }

    /// Server: This is what happened before you joined
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "recx")]
    pub struct RecapTail {
        pub start: usize,
        pub msgs: Vec<Value> }

    // Misc
    /// Server/Client: Echo
    #[derive(MessageDataV2)]
    #[msg_data(global=true, msg_type = "echo")]
    pub struct Echo(serde_json::Value);
}

impl From<&MessageV2> for String {
    fn from(val: &MessageV2) -> Self {
        serde_json::to_string(val).expect("Right now we do not handle errors in serialization")
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

impl From<MessageV2Enum> for String {
    fn from(val: MessageV2Enum) -> Self {
        let msg: &MessageV2 = &val.into();
        msg.into()
    }
}

pub trait IntoMessage {
    fn into_message(self) -> MessageV2;
    fn into_numbered_message(self, seq: u64) -> MessageV2
    where
        Self: Sized,
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
