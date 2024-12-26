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

use crate::lua::state_init;
use crate::state::{error_result, State};
use async_recursion::async_recursion;
use axum::extract::ws::{CloseFrame, Message as WsMessage, WebSocket};
use yapnet_core::lua::yapi::init_lua_from_argv;
use yapnet_core::prelude::Message;
use std::collections::HashMap;
use tokio::{
    select,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};
use yapnet_core::game::MessageResult;
use yapnet_core::prelude::*;

/// Server stored handle to the client task
pub struct ClientConnection {
    pub id: usize,
    pub to_client: Sender<ClientMessage>,
    client_handle: JoinHandle<()>,
}

type ClientMessage = String;

/// The client task and its state
pub struct Client {
    cid: usize,
    from_server: Receiver<ClientMessage>,
    to_server: Sender<Message>,
    remove_clients: Sender<CloseConnection>,
    websocket: WebSocket,
}

/// Client-task message to the server to close the channel and end the task
pub struct CloseConnection {
    pub id: usize,
    pub reason: CloseConnectionReason,
}

/// Reason for a client to close
pub enum CloseConnectionReason {
    ClientCloseFrame(CloseFrame<'static>),
    ClientCloseErr(axum::Error),
    ClientCloseEmpty,
}

/// Axum-side communication with the server.
/// Passed into AppState (And the client)
#[derive(Clone)]
pub struct ServerHandle {
    pub messages: Sender<Message>,
    pub add_clients: Sender<WebSocket>,
    pub remove_clients: Sender<CloseConnection>,
}

/// Websocket server task
pub struct Server {
    state: State,
    pub messages: Receiver<Message>,
    pub add_clients: Receiver<WebSocket>,
    pub remove_clients: Receiver<CloseConnection>,
    handle: ServerHandle,
    clients: HashMap<usize, ClientConnection>,
    users_connections: HashMap<usize, String>,
    // TODO: Assign id's more efficiently, maybe also use uuid here, or
    // remake the `clients` into a array and manage that.
    highest_id: usize,
}

impl Server {
    /// Make the server and the handle
    pub async fn create() -> (Self, ServerHandle) {
        let state = state_init(init_lua_from_argv());

        let (message_send, message_recv) = channel(128);
        let (add_clients_send, add_clients_recv) = channel(8);
        let (remove_clients_send, remove_clients_recv) = channel(8);

        let sh = ServerHandle {
            messages: message_send,
            add_clients: add_clients_send,
            remove_clients: remove_clients_send,
        };

        let sv = Server {
            messages: message_recv,
            add_clients: add_clients_recv,
            remove_clients: remove_clients_recv,
            clients: HashMap::new(),
            users_connections: HashMap::new(),
            state,
            handle: sh.clone(),
            highest_id: 0,
        };

        return (sv, sh);
    }

    /// The task that manages websocket connection
    pub async fn run(mut self) {
        loop {
            select! {
                client_opt = self.add_clients.recv() => {
                    let comm = client_opt.unwrap();
                    let id = self.highest_id;
                    self.highest_id += 1;

                    let client = ClientConnection::create_client(id,comm,&self.handle);
                    self.clients.insert(id, client);
                }
                client_opt = self.remove_clients.recv() => {
                    let close = client_opt.unwrap();
                    self.clients.remove(&close.id);
                    if let Some(uname)  = self.users_connections.remove(&close.id) {
                        let res = self.state.player_leave(&uname);
                        self.send_result(close.id as usize, res).await;
                    }
                    self.display_clients();
                    self.state.display_state();
                }
                msg_opt = self.messages.recv() => {
                    let msg = msg_opt.unwrap();
                    let cid = msg.seq;
                    println!("Message recieved!: {:?}", &msg);
                    let res = self.handle_message(msg);
                    self.send_result(cid as usize, res).await;
                }
            }
        }
    }

    fn display_clients(&self) {
        print!("Client_Connections: [");
        for (i, _) in self.clients.iter() {
            print!("{},", i)
        }
        println!("]");
        for (i, u) in self.users_connections.iter() {
            println!("[{}]: {}", i, u);
        }
    }

    pub fn handle_message(&mut self, m: Message) -> MessageResult {
        let cid = m.seq;
        match m.data {
            MessageData::BodyHello( Hello { username }) => match self.state.new_user(&username) {
                Ok(o) => {
                    self.users_connections
                        .insert(cid as usize, username.clone());
                    o
                }
                Err(o) => o,
            },
            MessageData::BodyBack( Back { token }) => match self.state.reauth_user(token) {
                Ok((username, o)) => {
                    self.users_connections
                        .insert(cid as usize, username.clone());
                    o
                }
                Err(o) => o,
            },
            _ => self.auth_handle_message(m),
        }
    }
    pub fn auth_handle_message(&mut self, m: Message) -> MessageResult {
        if let Some(username) = self.users_connections.get(&(m.seq as usize)) {
            self.state.handle_message(username, m)
        } else {
            error_result(
                "NotLoggedIn",
                "You need to be logged in to do this action.".to_string(),
                None,
            )
        }
    }

    #[async_recursion(?Send)]
    async fn try_serialize_send(&self, client: &ClientConnection, m: &Message) {
            let x = serde_json::to_string(&m);
            if let Ok(msg) = x {
                match client.to_client.send(msg).await {
                    Ok(()) => (),
                    Err(err) => eprintln!("[Send error] {:?}", err),
                    // TODO: Handle this error by disconnecting the player 
                }
            } else if let Err(err) = x{
                eprintln!("[Serialization Error] {:?}", err);
                self.send_result(client.id, MessageResult::Error( Error {
                    kind: "ESER".to_owned() ,
                    info: format!("Message serialization failed at seq: {}", m.seq),
                    details: "".to_string(),
                }.into_message())).await;
            }  
        
    }

    async fn send_result(&self, cid: usize, m: MessageResult) {
        match m {
            MessageResult::Error(m) | MessageResult::Return(m) => {
                if let Some(client) = self.clients.get(&cid) {
                    self.try_serialize_send(client, &m).await;
                }
            }
            MessageResult::BroadcastExclusive(m) => {
                for (cid2, client) in &self.clients {
                    if cid == *cid2 {
                        continue;
                    }
                    self.try_serialize_send(client, &m).await;
                    // TODO: Handle error and see what we can do about the clone
                }
            }
            MessageResult::Broadcast(m) => {
                for (_, client) in &self.clients {
                    self.try_serialize_send(client, &m).await;
                    // TODO: Handle error and see what we can do about the clone
                }
            }

            // Recursively send all the results
            MessageResult::Many(ms) => {
                for m in ms {
                    Box::pin(self.send_result(cid,m)).await
                }
            }

            MessageResult::None => (),
            MessageResult::Bulk(messages) => {
                let client = self.clients.get(&cid).unwrap();
                for m in messages{
                    self.try_serialize_send(client, &m).await;
                }
            }
        }
    }
}

impl ClientConnection {
    /// Creates a client connection and spawns the task
    pub fn create_client(id: usize, ws: WebSocket, handle: &ServerHandle) -> Self {
        let (to_client, from_server) = channel(8);

        let client = Client {
            cid: id,
            from_server,
            to_server: handle.messages.clone(),
            remove_clients: handle.remove_clients.clone(),
            websocket: ws,
        };

        let client_handle = tokio::spawn(client.run());

        ClientConnection {
            id,
            to_client,
            client_handle,
        }
    }
}

impl Client {
    /// Client task
    async fn run(mut self) {
        println!("Created a client!: {}", self.cid);
        loop {
            select! {
                recv = self.websocket.recv() => {
                    if let Some(m) = recv {
                        match m {
                            Ok(wsmsg) => {
                                match wsmsg {
                                    WsMessage::Text(json_msg) => {
                                        match serde_json::from_str::<Message>(&json_msg){
                                            Ok(mut msg) => {
                                                msg.seq = self.cid as u64;
                                                self.to_server.send(msg).await.unwrap()
                                            },
                                            Err(err) => {
                                                let msg = Message {
                                                    seq: 0,
                                                    data: Error{
                                                        kind: "InvalidMessage".to_string(),
                                                        info: format!("{:?}", err),
                                                        details: String::new(),
                                                    }.into()
                                                };
                                                if let Err(err) = self.websocket.send(WsMessage::Text(serde_json::to_string(&msg).expect("Serializing the message should never fail"))).await {
                                                    self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseErr(err)}).await.unwrap();
                                                };
                                            }
                                        }
                                    },
                                    WsMessage::Pong(pong) => println!("Pong! {:?}", pong),
                                    WsMessage::Close(close_opt) => {
                                        println!("Client sent a close frame, returning.");
                                        if let Some(close) = close_opt {
                                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseFrame(close)}).await.unwrap();
                                            return
                                        } else {
                                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseEmpty}).await.unwrap();
                                            return
                                        }
                                    },
                                    _ => todo!("Sent unsupported WS message!")
                                }
                            }
                            Err(err) => {
                                self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseErr(err)}).await.unwrap();
                                return
                            }
                        }
                    } else {
                        println!("There are no client packages left, returning.");
                        self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseEmpty}).await.unwrap();
                        return
                    }
                }
                    send = self.from_server.recv() => {
                    if let Some(m) = send {
                        if let Err(err) = self.websocket.send(WsMessage::Text(m)).await {
                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseErr(err)}).await.unwrap();
                        };
                    } else {
                        println!("Server closed the sender, closing connection!");
                        match self.websocket.close().await {
                            Ok(()) => (),
                            Err(err) => {
                                if err.to_string() != "Trying to work with closed connection"{
                                    println!("Websocket error! {}", err.to_string());
                                }
                            }
                        };
                        return;
                    }
                }
            }
        }
    }
}
