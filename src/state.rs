use std::collections::HashMap;
use uuid::Uuid;

use crate::Message;
use crate::MessageData;
type MessageView<'a> = Vec<&'a Message>;

struct MessageArr {
    inner: Vec<Message>,
    seq: u64,
}

impl MessageArr {
    fn new() -> Self {
        MessageArr {
            inner: vec![],
            seq: 0,
        }
    }

    /// This should be only done when sending messages.
    pub fn state_message(&mut self, m: MessageData) -> &Message {
        let s = self.seq;
        self.seq += 1;
        let message = Message { seq: s, data: m };
        self.inner.push(message);
        self.inner.last().expect("Just pushed")
    }

    pub fn push_and_serialize(&mut self, m: MessageData) -> String {
        let s = self.seq;
        self.seq += 1;
        let message = Message { seq: s, data: m };
        let d = serde_json::to_string(&message).unwrap();
        // TODO: Handle error
        self.inner.push(message);
        d
    }

    pub fn push_welcome_packet(
        &mut self,
        username: &String,
        user: &User,
        recap: MessageResult,
    ) -> MessageResult {
        let welcome = serialize_msg_data(MessageData::Welcome {
            username: username.clone(),
            token: user.uuid,
        });
        let player_joined = serialize_msg(self.state_message(MessageData::PlayerJoined {
            username: username.clone(),
        }));
        MessageResult::Many(vec![
            MessageResult::Return(welcome),
            MessageResult::BroadcastExclusive(player_joined),
            recap,
        ])
    }

    pub fn print_state(&self) {
        println!("---- [State] ----");
        for m in self.inner.iter() {
            println!("[{}] {:?}", m.seq, m.data)
        }
    }
}

pub struct State<'a> {
    messages: MessageArr,
    chats: HashMap<String, Chat<'a>>,
    users: HashMap<String, User<'a>>,
    /// Deprecated
    seq: u64,
}

#[derive(Debug)]
pub enum MessageResult {
    /// Send message to everyone
    Broadcast(String),
    /// Send message to everyone but the client who's message we are reacting too
    BroadcastExclusive(String),
    /// Error, only send the message to the one client
    Error(String),
    /// Only send the message to the one client who sent this message
    Return(String),
    /// Composite message for things like joining, leaving and recap
    Many(Vec<MessageResult>),
    /// Bulk messages, like Recaps
    Bulk(Vec<String>),
    /// Empty
    None,
}

struct Chat<'a> {
    messages: MessageView<'a>,
}

impl<'a> Chat<'_> {
    fn new() -> Self {
        Self { messages: vec![] }
    }

    fn check_player(&self, _: &User) -> bool {
        // Todo: Check permissions
        return true;
    }
}

pub struct User<'a> {
    pub online: bool,
    pub uuid: Uuid,
    messages: MessageView<'a>,
}

impl<'a> State<'_> {
    pub fn new() -> Self {
        let mut state = Self {
            messages: MessageArr::new(),
            chats: HashMap::new(),
            users: HashMap::new(),
            seq: 0,
        };

        state
            .chats
            .insert("general".to_string(), Chat { messages: vec![] });

        state
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
            | MessageData::RecapTail { .. } => {
                println!("Server side packet sent by client!");
                MessageResult::None
            }

            _ => todo!("Unsupported Message Type"),
        };
    }

    fn handle_chat(&mut self, sender: &String, m: Message) -> MessageResult {
        if let MessageData::ChatSend {
            chat_target,
            chat_content,
        } = m.data
        {
            let player = self.users.get(sender).expect("");
            if let Some(chat) = self.chats.get(&chat_target) {
                if chat.check_player(player) {
                    MessageResult::Broadcast(
                        serde_json::to_string(self.messages.state_message(MessageData::ChatSent {
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

                    //     Ok((
                    //         username.clone(),
                    //         self.push_welcome_packet(username, user)
                    //     ))
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
                let user = self.users.get(&username).unwrap(); // We already found the user before
                let recap = self.recap(&username, user);
                let welcome_packet = self.messages.push_welcome_packet(&username, user, recap);
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

        let user = User {
            uuid: token,
            online: true,
            messages: vec![],
        };

        self.users.insert(username.clone(), user);
        let userref = self.users.get(username).expect("We just added this user.");
        let recap = self.recap(username, userref);
        let welc = self.messages.push_welcome_packet(username, &userref, recap);

        Ok(welc)
    }
    pub fn player_leave(&mut self, userc: &String) -> MessageResult {
        if let Some(user) = self.users.get_mut(userc) {
            user.online = false;
            MessageResult::Broadcast(self.messages.push_and_serialize(MessageData::PlayerLeft {
                username: userc.clone(),
            }))
        } else {
            MessageResult::None
        }
    }

    pub fn display_state(&self) {
        self.messages.print_state()
    }

    const RECAP_CHUNK_SZ: usize = 64;

    fn recap(&self, username: &String, user: &User) -> MessageResult {
        let mut out = Vec::new();
        let mut mbuf = Vec::new();
        let mut start_cursor = 0;
        let mut chunks = 0;

        for m in self.messages.inner.iter() {
            let add = {
                m.data.is_global()
                    | match m.data.get_subject_player() {
                        Some(uname) => uname == *username,
                        None => false,
                    }
                    | match m.data.get_chat_name() {
                        None => false,
                        Some(chatn) => {
                            if let Some(_ch) = self.chats.get(&chatn) {
                                println!("recap chat message {:?}", chatn);
                                true
                                // ch.check_player(user)
                            } else {
                                false
                            }
                        }
                    }
            };

            if add {
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

        let head = MessageResult::Return(serialize_msg_data(MessageData::RecapHead {
            count: chunks,
            chunk_sz: Self::RECAP_CHUNK_SZ,
        }));

        MessageResult::Many(vec![
            head,
            MessageResult::Bulk(
                out.iter()
                    .map(|m| serde_json::to_string(m).unwrap())
                    .collect(),
            ),
        ])
    }
}

fn wrap_mdata(m: MessageData) -> Message {
    Message { seq: 0, data: m }
}

fn error_message(kind: &'static str, info: String, details: Option<String>) -> Message {
    wrap_mdata(MessageData::Error {
        kind: kind.to_string(),
        info,
        details: details.unwrap_or("{}".to_string()),
    })
}

pub fn serialize_msg(m: &Message) -> String {
    serde_json::to_string(m).expect("Right now we do not handle errors in serialization")
}

pub fn serialize_msg_data(m: MessageData) -> String {
    return serialize_msg(&wrap_mdata(m));
}

pub fn error_result(kind: &'static str, info: String, details: Option<String>) -> MessageResult {
    MessageResult::Error(
        serde_json::to_string(&wrap_mdata(MessageData::Error {
            kind: kind.to_string(),
            info,
            details: details.unwrap_or("{}".to_string()),
        }))
        .expect("Error serialization error!"),
    )
}
