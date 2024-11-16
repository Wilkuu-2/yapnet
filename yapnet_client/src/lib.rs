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

use core::panic;
use std::{cell::RefCell, collections::HashMap};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::{from_str, to_string};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WSMessage, MaybeTlsStream, WebSocketStream,
};
use yapnet_core::{
    game::chat::MessageRef,
    prelude::Chat,
    protocol::message::{Message, MessageData},
};

macro_rules! unpack_msg {
    {$e:expr, $t:pat => $b:block }=> {
      match &$e.data {
        $t => $b
        _ => unreachable!()
      }
    };
}

struct RecapInfo {
    current_seq: usize,
    end_chunk: usize,
    chunk_sz: usize,
}

pub struct GameState {
    pub messages: Vec<Message>,
    pub registered: bool,
    pub username: Option<String>,
    pub token: Option<uuid::Uuid>,
}

impl GameState {
    fn new() -> Self {
        Self {
            messages: vec![],
            registered: false,
            username: None,
            token: None,
        }
    }

    fn get_pending_index(&self) -> usize {
        return self.messages.len();
    }
}

pub struct PlayerState {
    pub username: String,
    pub connected: bool,
    pub role: String,
}

impl PlayerState {
    fn new(username: String) -> Self {
        Self {
            username,
            connected: true,
            role: "__default".to_string(),
        }
    }
}

pub struct LobbyState {
    pub players: HashMap<String, PlayerState>,
    pub chats: HashMap<String, Chat>,
}

fn blank_handler(_client: &Client, _msg: &Message) {}

impl LobbyState {
    fn new() -> Self {
        Self {
            chats: HashMap::new(),
            players: HashMap::new(),
        }
    }
}
pub struct Client {
    reader: SplitStream<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>,
    writer: SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, WSMessage>,
    pub state: GameState,
    pub lobby: LobbyState,
    recap_info: Option<RecapInfo>,
}

pub type ClientResult = Result<ClientAction, Error>;
// Holds the bool that decides if the message is saved to messages
struct ClientResultOuter(ClientResult, bool);
pub enum ClientAction {
    None,
    Welcome,
    PlayerJoined(String),
    PlayerLeft(String),
    Chat(MessageRef),
    RecapEnd,
    Error(String),
    Multiple(Vec<ClientResult>),
}

impl Client {
    pub async fn connect(url: String) -> Result<Self, Error> {
        let (stream, _response) = connect_async(url).await.map_err(|e| Error::Websocket(e))?;
        // TODO: Validate connection more
        //
        let (writer, reader) = stream.split();

        Ok(Self {
            writer,
            reader,
            state: GameState::new(),
            lobby: LobbyState::new(),
            recap_info: None,
        })
    }

    async fn send_message_pre(&mut self, msg: MessageData) -> Result<(), Error> {
        let wrapped = Message { seq: 0, data: msg };
        self.writer
            .send(WSMessage::Text(
                to_string(&wrapped).expect("Serialization should never fail here"),
            ))
            .await
            .map_err(|e| Error::Websocket(e))
    }

    pub async fn send_register(&mut self, username: String) {
        self.send_message_pre(MessageData::Hello { username })
            .await
            .unwrap()
    }

    pub async fn send_message(&mut self, msg: MessageData) -> Result<(), Error> {
        match self.state.registered {
            true => self.send_message_pre(msg).await,
            false => Err(Error::Unregistered),
        }
    }

    pub async fn send_login(&mut self, token: uuid::Uuid) {
        self.send_message_pre(MessageData::Back { token })
            .await
            .unwrap();
    }

    pub async fn recieve_and_handle(&mut self) -> ClientResult {
        match self.reader.next().await.unwrap() {
            Ok(msg) => self.handle_ws(msg),
            Err(e) => Err(Error::Websocket(e)),
        }
    }

    fn handle_ws(&mut self, wsm: WSMessage) -> ClientResult {
        match wsm {
            WSMessage::Text(tx) => self.handle_message(from_str(&tx).unwrap()),
            WSMessage::Ping(_) | WSMessage::Pong(_) => Ok(ClientAction::None),
            _ => todo!(),
        }
    }

    fn handle_message(&mut self, msg: Message) -> ClientResult {
        let ret: ClientResultOuter = match msg.data {
            MessageData::Error {
                ref kind,
                info: _,
                ref details,
            } => ClientResultOuter(
                Ok(ClientAction::Error(format!("{} => {}", kind, details))),
                false,
            ),
            MessageData::PlayerJoined { .. } => self.handle_player_joined(&msg),
            MessageData::PlayerLeft { .. } => self.handle_player_left(&msg),
            MessageData::ChatSent { .. } => self.handle_chat(&msg),
            MessageData::Welcome { .. } => self.handle_welcome(&msg),
            MessageData::Setup { .. } => self.handle_setup(&msg),
            MessageData::RecapHead { .. } => self.start_recap(&msg),
            MessageData::RecapTail { .. } => self.progress_recap(&msg),
            d @ MessageData::ChatSend { .. }
            | d @ MessageData::Hello { .. }
            | d @ MessageData::Back { .. }
            | d @ MessageData::Echo(..) => panic!("Message for server sent here: {:?}", d),
        };

        if ret.1 {
            self.state.messages.push(msg);
        }
        ret.0
    }

    fn handle_player_joined(&mut self, msg: &Message) -> ClientResultOuter {
        assert!(self.state.registered);
        unpack_msg! {msg, MessageData::PlayerJoined { username } => {
            let uname = username.clone();
            if uname != self.state.username.clone().unwrap() {
                if let Some(player) = self.lobby.players.get_mut(username) {
                    if player.connected {
                        eprintln!("Double player connection")
                    }
                    player.connected = true;
                } else {
                    let player = PlayerState::new(uname.clone());
                    self.lobby.players.insert(uname.clone(), player);
                }
            }
            ClientResultOuter(Ok(ClientAction::PlayerJoined(uname)), true)
        }}
    }
    fn handle_player_left(&mut self, msg: &Message) -> ClientResultOuter {
        assert!(self.state.registered);
        unpack_msg! {msg, MessageData::PlayerLeft { username } => {
            let uname = username.clone();
            if uname != self.state.username.clone().unwrap() {
                if let Some(player) = self.lobby.players.get_mut(username) {
                    if !player.connected {
                        eprintln!("Double player disconnection")
                    }
                    player.connected = false;
                } else {
                    let player = PlayerState::new(uname.clone());
                    self.lobby.players.insert(uname.clone(), player).unwrap();
                }
            }
            ClientResultOuter(Ok(ClientAction::PlayerJoined(uname)), true)
        }}
    }
    fn handle_welcome(&mut self, msg: &Message) -> ClientResultOuter {
        unpack_msg! {msg, MessageData::Welcome { username, token } => {
            self.state.username = Some(username.clone());
            self.state.token = Some(token.clone());
            self.state.registered = true;
        }};
        ClientResultOuter(Ok(ClientAction::Welcome), false)
    }
    fn handle_chat(&mut self, msg: &Message) -> ClientResultOuter {
        unpack_msg! {msg, MessageData::ChatSent { chat_sender: _, chat_target , chat_content: _ } => {
            let ind = self.state.get_pending_index();
            self.lobby.chats.get_mut(chat_target).unwrap().messages.push(ind);
            ClientResultOuter(Ok(ClientAction::Chat(ind)), true)
        }}
    }
    fn handle_setup(&mut self, msg: &Message) -> ClientResultOuter {
        unpack_msg! {msg, MessageData::Setup {chats} => {
            for chat in chats {
                self.lobby.chats.insert(chat.name.clone(), Chat::new(chat.perm.clone()));
            }
        }};
        ClientResultOuter(Ok(ClientAction::None), true)
    }
    fn start_recap(&mut self, msg: &Message) -> ClientResultOuter {
        unpack_msg! {msg, MessageData::RecapHead { count, chunk_sz } => {
            self.recap_info = Some(RecapInfo { chunk_sz: *chunk_sz, end_chunk: *count , current_seq: 0 })
        }
        };
        ClientResultOuter(Ok(ClientAction::None), false)
    }
    fn progress_recap(&mut self, msg: &Message) -> ClientResultOuter {
        let mut actions: Vec<ClientResult> = vec![];
        unpack_msg! {msg, MessageData::RecapTail { start, msgs } => {
            let (end, mut current) = match &self.recap_info {
                Some(recap) => (recap.end_chunk, recap.current_seq),
                None => { return ClientResultOuter(Err(Error::NoRecapHead) ,false);} ,

            };

            assert_eq!(*start, current);
            for msgv in msgs {
                let msg: Message = serde_json::from_value(msgv.clone()).unwrap();

                // debug_assert_eq!(msg.seq as usize, current);

                // Todo: Make sure that the chunks are counted properly
                // debug_assert!((msg.seq as usize) <= end);

                current += 1;

                actions.push(self.handle_message(msg))
            }


            self.recap_info.as_mut().unwrap().current_seq = current;
        }};

        let recap = self.recap_info.as_ref().unwrap();
        if recap.current_seq >= recap.end_chunk {
            actions.push(Ok(ClientAction::RecapEnd));
            self.recap_info = None;
        }

        ClientResultOuter(Ok(ClientAction::Multiple(actions)), false)
    }
}

#[derive(Debug)]
pub enum Error {
    Unregistered,
    Websocket(tungstenite::Error),
    NoRecapHead,
}
