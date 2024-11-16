// Copyright 2024 Jakub Stachurski
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

use mlua::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::lua::{LuaState, StateFrame};

use yapnet_core::prelude::*;

pub struct State {
    history: History,
    pub(crate) lua_state: Option<LuaState>,
    pub chats: Chats,
    pub users: Users,
}

impl State {
    pub fn new() -> Self {
        Self {
            history: History::new(),
            chats: Chats::new(),
            users: Users::new(),
            lua_state: None,
        }
    }

    pub fn push_setup_message(&mut self) {
        let mut chats = vec![];
        for (name, v) in self.chats.iter() {
            chats.push(ChatSetup {
                name: name.clone(),
                perm: v.perms.clone(),
            })
        }

        self.history.push(MessageData::Setup { chats })
    }

    pub fn handle_message(&mut self, username: &String, m: Message) -> MessageResult {
        return match m.data {
            MessageData::Back { .. } | MessageData::Hello { .. } => {
                unreachable!("Back and Hello should be already handled")
            }

            MessageData::ChatSend { .. } => self.handle_chat(username, m),

            MessageData::Welcome { .. }
            | MessageData::ChatSent { .. }
            | MessageData::PlayerLeft { .. }
            | MessageData::PlayerJoined { .. }
            | MessageData::RecapHead { .. }
            | MessageData::RecapTail { .. }
            | MessageData::Setup { .. } => {
                println!("Server side packet sent by client!");
                MessageResult::None
            }
            MessageData::Echo(_) => todo!("echo"),
            MessageData::Error { .. } => todo!("error"),
        };
    }

    fn lua_call<'lua, A>(
        &'lua self,
        callback_name: &'static str,
        args: A,
    ) -> LuaResult<Arc<Mutex<StateFrame>>>
    where
        A: IntoLuaMulti<'lua>,
    {
        match &self.lua_state {
            Some(lua_state) => {
                let frame = Arc::new(Mutex::new(StateFrame::make(self)));
                lua_state.callback(
                    callback_name,
                    frame.clone(),
                    args.into_lua_multi(&lua_state.lua)?,
                );
                Ok(frame)
            }
            None => Err(mlua::Error::SerializeError("There is no lua!".to_string())),
        }
    }

    fn handle_chat(&mut self, sender: &String, m: Message) -> MessageResult {
        if let MessageData::ChatSend {
            chat_target,
            chat_content,
        } = m.data
        {
            let player = self.users.get(sender).expect("");
            if let Some(chat) = self.chats.get(&chat_target) {
                if chat.can_write(player) {
                    match self.lua_call(
                        "on_chat",
                        (chat_target.clone(), sender.clone(), chat_content.clone()),
                    ) {
                        Ok(frame) => {
                            let fr = frame.lock().unwrap();
                            println!("{:?}", fr.outbound)
                        }
                        Err(e) => eprintln!("on_chat failed: {}", e),
                    }

                    MessageResult::Broadcast(
                        serde_json::to_string(self.history.state_message(MessageData::ChatSent {
                            chat_sender: sender.clone(),
                            chat_target,
                            chat_content,
                        }))
                        .expect("Chat serialization failure"),
                    )
                } else {
                    error_result(
                        "ChatPermDeny",
                        "Sending to this chat is denied".to_string(),
                        None,
                    )
                }
            } else {
                error_result(
                    "ChatNotFound",
                    format!("Chat {} not found!", chat_target),
                    Some(format!(r#"{{"chat_target":"{}"}}"#, chat_target)),
                )
            }
        } else {
            unreachable!("handle_chat is always used with ChatSend packets")
        }
    }

    pub fn reauth_user(&mut self, token: Uuid) -> Result<(String, MessageResult), MessageResult> {
        let res1: Result<String, MessageResult> = {
            if let Some((username, user)) = self.users.iter_mut().find(|u| u.1.uuid == token) {
                if !user.online {
                    user.online = true;
                    Ok(username.clone())
                } else {
                    Err(error_result(
                        "AlreadyLoggedIn",
                        "The token holder is already logged in".to_string(),
                        None,
                    ))
                }
            } else {
                Err(error_result(
                    "InvalidToken",
                    "The token you gave is not valid".to_string(),
                    None,
                ))
            }
        };

        match res1 {
            Err(err) => Err(err),
            Ok(username) => {
                let uuid = self
                    .users
                    .get(&username)
                    .expect("The user was already located before")
                    .uuid; // TODO: We already found the user before in the code above, why can't we use that result instead?
                let welcome_packet = self.successful_login(&username, uuid);
                Ok((username, welcome_packet))
            }
        }
    }
    pub fn new_user(&mut self, username: &String) -> Result<MessageResult, MessageResult> {
        if let Some(_) = self.users.get(username) {
            return Err(error_result(
                "UsernameTaken",
                format!("Username: {} already taken", username),
                None,
            ));
        }

        let token = Uuid::new_v4();
        let user = User::new(token);
        self.users.insert(username.clone(), user);
        Ok(self.successful_login(username, token))
    }
    pub fn player_leave(&mut self, userc: &String) -> MessageResult {
        if let Some(user) = self.users.get_mut(userc) {
            user.online = false;
            MessageResult::Broadcast(self.history.push_and_serialize(MessageData::PlayerLeft {
                username: userc.clone(),
            }))
        } else {
            MessageResult::None
        }
    }

    pub fn display_state(&self) {
        self.history.print_state()
    }

    const RECAP_CHUNK_SZ: usize = 64;

    fn user_can_view(&self, Message { data, .. }: &Message, username: &String) -> bool {
        if data.is_global() {
            return true;
        } else if let Some(uname) = data.get_subject_player() {
            if uname == *username {
                return true;
            }
        } else if let Some(chatn) = data.get_chat_name() {
            if let Some(ch) = self.chats.get(&chatn) {
                if ch.can_read(
                    self.users
                        .get(username)
                        .expect("Assumed that the user exists if their visibility is checked."),
                ) {
                    return true;
                }
            }
        }
        true
    }

    fn recap(&self, username: &String) -> MessageResult {
        let mut out = Vec::new();
        let mut mbuf = Vec::new();
        let mut start_cursor = 0;
        let mut chunks = 0;

        for m in self.history.iter() {
            if self.user_can_view(m, username) {
                if mbuf.len() < Self::RECAP_CHUNK_SZ {
                    mbuf.push(m);
                } else {
                    chunks += 1;

                    out.push(Message {
                        seq: chunks as u64,
                        data: MessageData::RecapTail {
                            start: start_cursor,
                            msgs: mbuf
                                .iter()
                                .map(|m| serde_json::to_value(m).unwrap())
                                .collect(),
                        },
                    });

                    start_cursor += mbuf.len();
                    mbuf = Vec::new();
                }
            }
        }
        // Append at the end
        if mbuf.len() > 0 {
            chunks += 1;
            out.push(Message {
                seq: chunks as u64,
                data: MessageData::RecapTail {
                    start: start_cursor,
                    msgs: mbuf
                        .iter()
                        .map(|m| serde_json::to_value(m).unwrap())
                        .collect(),
                },
            });
        }

        let head = MessageResult::Return(
            MessageData::RecapHead {
                count: chunks,
                chunk_sz: Self::RECAP_CHUNK_SZ,
            }
            .into(),
        );

        MessageResult::Many(vec![
            head,
            MessageResult::Bulk(
                out.iter()
                    .map(|m| serde_json::to_string(m).unwrap())
                    .collect(),
            ),
        ])
    }

    fn successful_login(&mut self, username: &String, uuid: Uuid) -> MessageResult {
        let recap = self.recap(username);
        let welcome = MessageData::Welcome {
            username: username.clone(),
            token: uuid,
        }
        .into();
        let player_joined = self.history.push_and_serialize(MessageData::PlayerJoined {
            username: username.clone(),
        });
        MessageResult::Many(vec![
            MessageResult::Return(welcome),
            MessageResult::BroadcastExclusive(player_joined),
            recap,
        ])
    }
}

fn error_message(kind: &'static str, info: String, details: Option<String>) -> Message {
    MessageData::Error {
        kind: kind.to_string(),
        info,
        details: details.unwrap_or("{}".to_string()),
    }
    .into()
}

pub fn error_result(kind: &'static str, info: String, details: Option<String>) -> MessageResult {
    MessageResult::Error(
        MessageData::Error {
            kind: kind.to_string(),
            info,
            details: details.unwrap_or("{}".to_string()),
        }
        .into(),
    )
}
