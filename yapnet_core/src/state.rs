// Copyright 2025 Jakub Stachurski
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


use std::{mem, ptr::from_ref};
use uuid::Uuid;
use crate::{error::{ClientError, ServerError}, lua::LuaState, models::history::{self, History}, prelude::{MessageV2Enum as MessageData, *}, protocol::{ChatId, UserId}};



pub struct ResponseFrame{
    history: History,
    responses: Vec<YapnetResponse>,
    ephemeral_messages: Vec<Message>,
} 

impl ResponseFrame {
    pub fn from_state(state: &YapnetState, cap: usize) -> Self {
         Self::new(&state.history, cap)
    }

    pub fn new(history: &History, cap: usize) -> Self {
        Self {
            history: history.get_frame(), 
            responses: Vec::with_capacity(cap), 
            ephemeral_messages: Vec::new(),
        } 
    }

    pub fn broadcast(&mut self, msg: MessageData, chat: ChatId) {
        let packet = self.history.state_message(msg);
        self.responses.push(YapnetResponse::Broadcast(packet.seq, chat))
    }
    pub fn broadcast_ex(&mut self, msg: MessageData, chat: ChatId) {
        let packet = self.history.state_message(msg);
        self.responses.push(YapnetResponse::BroadcastExclusive(packet.seq, chat))
    }
    pub fn error(&mut self, error: ClientError) {
        let i = self.ephemeral_messages.len();
        self.ephemeral_messages.push(error.clone().into_message()); 
        self.responses.push(YapnetResponse::Return(i))
    } 
    pub fn ret(&mut self, msg: MessageData) {
        let i = self.ephemeral_messages.len();
        self.ephemeral_messages.push(msg.into_message()); 
        self.responses.push(YapnetResponse::Return(i));
    } 

    pub fn ret_all(&mut self, recap: Vec<MessageData>) {
        let mut i = self.ephemeral_messages.len();
        self.responses.reserve(recap.len());
        self.ephemeral_messages.reserve(recap.len());
        for r in recap.into_iter() {
            self.responses.push(YapnetResponse::Return(i));
            self.ephemeral_messages.push(r.into());
            i += 1; 
        } 
    }

    pub fn is_ephemeral(&self) -> bool {
        self.history.is_empty()
    } 

    pub fn fetch_pair<'r>(&'r self, index: usize) -> Option<(&'r YapnetResponse, &'r Message)>{
        if let Some(response) = self.responses.get(index){
            match &response {
                YapnetResponse::Broadcast(seq,_) | YapnetResponse::BroadcastExclusive(seq,_) => {
                    let msg = self.history.get_message(*seq).expect("Responses should always have a matching message");
                    Some((response, msg))

                },
                YapnetResponse::None => { unreachable!() },
                YapnetResponse::Return(id,) => {
                    let msg = self.ephemeral_messages.get(*id).expect("Responses should always have a matching message");
                    Some((response, msg))
                } 
            }

        } else {
            None
        }
    }
} 

pub struct ResponseViewIter<'r> {
    frame: &'r ResponseView<'r>,
    response_index: usize,
}

impl<'r> ResponseViewIter<'r> {
    pub fn new<'v>(rv: &'r ResponseView<'v>) -> Self 
    where 'v:'r 
    {
        Self {
            frame: rv, 
            response_index: 0,
        }

    }
}

impl<'r> Iterator for ResponseViewIter<'r> {
    type Item = (&'r YapnetResponse, &'r Message);

    fn next(&mut self) -> Option<Self::Item> {
       let i = self.frame.responses.len();
       if self.response_index >= i {
            None
       } else {
            let re = self.response_index; 
            self.response_index += 1; 
            let response = self.frame.responses.get(re).unwrap(); 
            let message =  match response {
                YapnetResponse::Broadcast(seq, _) |
                YapnetResponse::BroadcastExclusive(seq, _) => { 
                    if self.frame.history.is_some() { 
                        self.frame.history.as_slice()[0].get_message(*seq)
                    } else {
                        // No history so skipping this value
                        // TODO: Logging
                        return self.next()
                    }
                },
                YapnetResponse::Return(id) =>  self.frame.ephemeral_messages.get(*id),
                YapnetResponse::None => unreachable!(),
            }.expect("Responses should always have matching messages!");
            Some((response, message))
       }
    }
}



#[derive(Debug, Clone)]
pub enum YapnetResponse {
    /// Send message to everyone
    Broadcast(u64, ChatId),
    /// Send message to everyone but the client who's message we are reacting too
    BroadcastExclusive(u64, ChatId),
    /// Only send the message to the one client who sent this message
    /// Used for errors
    /// Does not get pushed into the history
    Return(usize),
    /// Empty
    /// Does not get pushed into the history
    None,
}

#[derive(Debug, Clone)]
pub struct ResponseView<'s> {
    history: Option<&'s History>,
    responses: Vec<YapnetResponse>,
    ephemeral_messages: Vec<Message>,
} 

impl<'s> ResponseView<'s> {
    pub fn new(history: &'s History, capacity: usize) -> Self {
        Self {
            history: Some(history), 
            responses: Vec::with_capacity(capacity),
            ephemeral_messages: Vec::with_capacity(capacity),
        }
    }

    fn new_ephemeral(capacity: usize) -> Self {
        Self {
            history: None, 
            responses: Vec::with_capacity(capacity),
            ephemeral_messages: Vec::with_capacity(capacity),
        }
    } 

    pub fn from_message_return<T: IntoMessage>(msg: T) -> Self {
        let mut s = Self::new_ephemeral(1); 
        s.ephemeral_messages.push(msg.into_message());
        s.responses.push(YapnetResponse::Return(0));
        s
    }

    pub fn is_ephemeral(&self) -> bool {
        self.history.is_some()
    } 

    fn push_frame<'a>(&mut self, frame: ResponseFrame, mut_history: &'a mut History) where 
        's: 'a 
    {
        mut_history.merge(frame.history);
        let i = self.ephemeral_messages.len(); 
        self.responses.extend(frame.responses.into_iter().map(|r| match r {
            YapnetResponse::Return(id) => YapnetResponse::Return(id + i),
            x => x,
        }));
        self.ephemeral_messages.extend(frame.ephemeral_messages);
    }

    fn set_history(&mut self, history: &'s History) {
        self.history = Some(history);
    }

    pub fn iter<'r>(&'s self) -> ResponseViewIter<'r> where 's:'r {
        ResponseViewIter::new(self)
    }
}

pub struct YapnetState {
    pub lua_state: Option<LuaState>, 
    outbound: Vec<ResponseFrame>,
    history: History,
    pub chats: Chats, 
    pub users: Users, 
} 

impl YapnetState {
    const RECAP_CHUNK_SZ: usize = 64;
    pub fn new() -> Self {
        Self {
            lua_state: None,
            outbound: Vec::new(),
            history: History::new(),
            chats: Chats::new(),
            users: Users::new(),
        }
    }

    pub fn consume_frames<'s,'a>(&'s mut self) -> ResponseView<'a> 
    where  
       'a:'s 
    {
        let cap = self.outbound.capacity();
        let out = mem::replace(&mut self.outbound, Vec::with_capacity(cap)); 
        let mut view = ResponseView::new_ephemeral(self.outbound.len()*2);
        for frame in out.into_iter() {
            view.push_frame(frame, &mut self.history);
        }
    
        view.set_history(unsafe { mem::transmute::<&'s History ,&'a History>(&self.history) } );
        view.clone() 
    }

    

    pub fn handle_message_serveir<'s>(&'s mut self, username: &String, m: Message) -> ResponseView<'s>{
        match m.data {
            MessageData::BodyBack { .. } | MessageData::BodyHello { .. } => {
                unreachable!("Back and Hello should be already handled")
            }
            MessageData::BodyChatSend { .. } => self.handle_chat(username, m),
            MessageData::BodyWelcome { .. }
            | MessageData::BodyChatSent { .. }
            | MessageData::BodyPlayerLeft { .. }
            | MessageData::BodyPlayerJoined { .. }
            | MessageData::BodyRecapHead { .. }
            | MessageData::BodyRecapTail { .. }
            | MessageData::BodySetup { .. } => {
                eprintln!("Server side packet sent by client!");
            }
            MessageData::BodyEcho(_) => todo!("echo"),
            MessageData::BodyYnError { .. } => todo!("error"),
            x => {
                eprintln!(
                    "Unknown protocol message found {} ",
                    x.to_inner().msg_type()
                );
            }
        }
        self.consume_frames()
    }
    
    fn handle_chat(&mut self, sender: &String, m: Message) {
        if let MessageData::BodyChatSend(ChatSend{
            chat_target,
            chat_content,
        }) = m.data {
            let mut frame = ResponseFrame::new(&self.history, 2); 
            let player = self.users.get(sender).expect("");
            if let Some(chat) = self.chats.get(&chat_target) {
                if chat.can_write(player) {
                    frame.broadcast(
                                ChatSent {
                                    chat_sender: sender.clone(),
                                    chat_target: chat_target.clone(),
                                    chat_content,
                                }
                                .into(),
                        sender.clone());
                } else { 
                    frame.error(ClientError::NoPermission(chat_target,"".to_string()));
                }
            } else {  
                frame.error(ClientError::InvalidChat(chat_target,"Not found".to_string()));
            }
            self.outbound.push(frame);
        } else {
            unreachable!("handle_chat is always used with ChatSend packets")
        }
    }
    pub fn reauth_user(&mut self, token: Uuid) -> Result<(String,ResponseView<'_>), ServerError> {
        let mut frame = ResponseFrame::new(&self.history, 8);
        let uname = if let Some((username, user)) = self.users.iter_mut().find(|u| u.1.uuid == token) {
            if !user.online {
                user.online = true;
                username.clone()
            } else {
                return Err(ServerError::AlreadyJoinedOrLeft)
            }
        } else {
            return Err(ServerError::InvalidToken)
        };

        self.successful_login(&mut frame, &uname, token);
        Ok((uname.clone() ,self.consume_frames()))
    }
    
    fn successful_login(&mut self, frame: &mut ResponseFrame, username: &String, uuid: Uuid) {
        let recap = self.recap(username);
        let welcome = Welcome {
            username: username.clone(),
            token: uuid,
        }.into();

        let player_joined = 
                PlayerJoined {
                    username: username.clone(),
                }.into(); 


        frame.ret(welcome);
        frame.ret_all(recap); 
        frame.broadcast_ex(player_joined, "system:all".to_string());
    }
    
    fn recap(&self, username: &String) -> Vec<MessageV2Enum>{
        let mut out = Vec::new();
        let mut mbuf = Vec::new();
        let mut start_cursor = 0;
        let mut chunks = 0;

        out.push(RecapHead {count: 0 , chunk_sz: Self::RECAP_CHUNK_SZ}.into()); 
        for m in self.history.iter() {
            if self.user_can_view(m, username) {
                if mbuf.len() < Self::RECAP_CHUNK_SZ {
                    mbuf.push(m);
                } else {
                    chunks += 1;

                    out.push(RecapTail {
                            start: start_cursor,
                            msgs: mbuf
                                .iter()
                                .map(|m| serde_json::to_value(m).unwrap())
                                .collect(),
                            }.into());

                    start_cursor += mbuf.len();
                    mbuf = Vec::new();
                }
            }
        }
        // Append at the end
        if !mbuf.is_empty() {
            chunks += 1;
            out.push(RecapTail {
                    start: start_cursor,
                    msgs: mbuf
                        .iter()
                        .map(|m| serde_json::to_value(m).unwrap())
                        .collect(),
                    }.into());
        }
       
        // Accessing the item in the array, praying for the compiler to just get the first element
        // and change that one variable.
        match &mut out.get_mut(0).expect("We just pushed") {
            MessageV2Enum::BodyRecapHead(h) => {h.count = chunks}
            _ => unreachable!(),
        }
        
        out
    }
    
    fn user_can_view(&self, msg: &Message, username: &String) -> bool {
        let obj = msg.data.to_inner_ref();

        if obj.is_global() {
            return true;
        } else if let Some(uname) = obj.subject() {
            if uname == *username {
                return true;
            }
        } else if let Some(chatn) = obj.chat() {
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

    pub fn return_response(&mut self, msg: MessageV2Enum) -> ResponseView<'_> {
        let mut frame = ResponseFrame::new(&self.history, 1); 
        frame.ret(msg);
        self.consume_frames() 
    }

    pub fn new_user<'a>(&'a mut self, username: &String) ->  Result<ResponseView<'a>, ServerError> {
        if self.users.contains_key(username) {
            return Err(ServerError::NameTaken(username.clone())); 
        }

        let token = Uuid::new_v4();
        let user = User::new(token);
        let mut frame = ResponseFrame::new(&self.history, 8);

        self.users.insert(username.clone(), user);
        self.successful_login(&mut frame, username, token);
        self.outbound.push(frame);
        Ok(self.consume_frames())
    }
    pub fn player_leave<'a>(&'a mut self, userc: &String) -> Result<ResponseView<'a>,ServerError> {
        if let Some(user) = self.users.get_mut(userc) {
            let mut frame = ResponseFrame::new(&self.history,1);
            user.online = false;
            frame.broadcast(PlayerLeft {
                    username: userc.clone(),
                }.into(),
                    "system:all".to_string(),
                );
            Ok(self.consume_frames())
        } else {
            Err(ServerError::AlreadyJoinedOrLeft)
        }
    }

    pub fn push_setup_message(&mut self) {
        self.history.state_message(Setup {
            chats: self.chats.iter().map(|(name, chat)| {
               ChatSetup {
                    name: name.clone(),
                    perm: chat.perms.clone(),
                }  
            }).collect(),
        }.into());

    }

    pub fn print_messages(&self) {
        self.history.print_state();
    }
} 
