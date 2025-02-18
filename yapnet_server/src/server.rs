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

use yapnet_core::lua::state_init;
use async_recursion::async_recursion;
use axum::extract::ws::{CloseFrame, Message as WsMessage, WebSocket};
use yapnet_core::error::ClientError;
use yapnet_core::protocol::ChatId;
use yapnet_core::state::{ResponseView, YapnetResponse};
use std::collections::HashMap;
use tokio::{
    select,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};
use yapnet_core::lua::yapi::init_lua_from_argv;
use yapnet_core::prelude::Message;
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
    Frame(CloseFrame<'static>),
    Err(axum::Error),
    Empty,
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
    state: yapnet_core::state::YapnetState,
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

        (sv, sh)
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
                        // Bypassing the borrow checker, since we know that the reference in res in
                        // immutable.
                        let ptr = std::ptr::from_ref(&self);
                        let res = self.state.player_leave(&uname).unwrap();
                        unsafe { (*ptr).send_result(close.id, res).await };
                    }
                    self.display_clients();
                }
                msg_opt = self.messages.recv() => {
                    let msg = msg_opt.unwrap();
                    let cid = msg.seq;
                    println!("Message recieved!: {:?}", &msg);
                    // Bypassing the borrow checker, since we know that the reference in res in
                    // immutable.
                    let ptr = std::ptr::from_ref(&self);
                    let res = self.handle_message(msg);
                    unsafe { 
                        let refr: &Server = &(*ptr); 
                        refr.send_result(cid as usize, res).await;
                        refr.state.print_messages();
                    }
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

    pub fn handle_message<'s,'v>(&'s mut self, m: Message) -> ResponseView<'v> 
        where 's:'v 
    {
        let cid = m.seq;
        match m.data {
            MessageData::BodyHello(Hello { username }) => {  
                match self.state.new_user(&username) {
                    Ok(frame) => {
                        self.users_connections
                            .insert(cid as usize, username.clone());
                        frame
                    },
                    Err(e) => {
                        ResponseView::from_message_return(e)
                    },
                }
            },
            MessageData::BodyBack(Back { token }) =>
                match self.state.reauth_user(token) {
                    Ok((username,frame)) => {
                        self.users_connections
                            .insert(cid as usize, username);
                        frame
                    },
                    Err(e) => {
                        ResponseView::from_message_return(e)
                    },
                }
            _ => self.auth_handle_message(m),
        }
    }
    pub fn auth_handle_message(&mut self, m: Message) -> ResponseView<'_>{
        if let Some(username) = self.users_connections.get(&(m.seq as usize)) {
            self.state.handle_message_serveir(username, m)
        } else {
            ResponseView::from_message_return(ClientError::NoLogin)
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
        } else if let Err(err) = x {
            match client.to_client.send(serde_json::to_string(&(YnError::new("SerializationError","The message you sent was malformed",format!("{{ \"SerdeError\":\"{}\"}}",err)).into_message())).unwrap()).await {
                Ok(_) => {},
                Err(e) => {eprintln!("Send error: {}", e)},
            };
        }
    }

    async fn try_serialize_send_all_ex<'a,'s, T: Iterator<Item = (&'a usize, &'a ClientConnection)>>(&'s self, m: &Message,cid_ex: usize ,  iter: T) {
        for (cid2, client) in iter {
            if cid_ex == *cid2 {
                continue;
            }
            self.try_serialize_send(client, m).await;
            // TODO: Handle error and see what we can do about the clone
        }
    }
    async fn try_serialize_send_all<'a,'s, T: Iterator<Item = (&'a usize, &'a ClientConnection)>>(&'s self, m: &Message, iter: T) {
        for (_, client) in iter {
            self.try_serialize_send(client, m).await;
            // TODO: Handle error and see what we can do about the clone
        }
    }

    // TODO: rewrite as iter?
    fn all_participating_clients<'s,'i>(&'s self, chatid: &ChatId) -> Vec<(&usize,&ClientConnection)> 
    where 
      's: 'i
    {
           match self.state.chats.get(chatid) {
                None => vec![],
                Some(chat) =>  {
                   self.users_connections.iter().filter_map(|(k,v)| {
                        match chat.can_read(self.state.users.get(v).expect("users_connections should be a subset of state.users ")) {
                            true => self.clients.get(k).map(|c| (k, c)),
                            false => None,
                        }
                    }).collect() 
                } 
           } 
    }

    async fn send_result<'a>(&self, cid: usize, rv: ResponseView<'a>) {
        for (resp,m) in rv.iter() {
            match resp {
                YapnetResponse::Return(_) => {
                    if let Some(client) = self.clients.get(&cid) {
                        self.try_serialize_send(client, m).await;
                    }
                }
                YapnetResponse::BroadcastExclusive(_, chat) => {
                    if chat == "system:all" {
                        self.try_serialize_send_all_ex(m , cid, self.clients.iter()).await;
                    } else {
                        self.try_serialize_send_all_ex(m, cid, self.all_participating_clients(chat).into_iter()).await; 
                    }
                }
                YapnetResponse::Broadcast(_, chat) => {
                    if chat == "system:all" {
                        self.try_serialize_send_all(m ,self.clients.iter()).await;
                    } else {
                        self.try_serialize_send_all(m, self.all_participating_clients(chat).into_iter()).await; 
                    }
                }
                YapnetResponse::None => {} 
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
                                                    data: YnError{
                                                        kind: "InvalidMessage".to_string(),
                                                        info: format!("{:?}", err),
                                                        details: String::new(),
                                                    }.into()
                                                };
                                                if let Err(err) = self.websocket.send(WsMessage::Text(serde_json::to_string(&msg).expect("Serializing the message should never fail"))).await {
                                                    self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::Err(err)}).await.unwrap();
                                                };
                                            }
                                        }
                                    },
                                    WsMessage::Pong(pong) => println!("Pong! {:?}", pong),
                                    WsMessage::Close(close_opt) => {
                                        println!("Client sent a close frame, returning.");
                                        if let Some(close) = close_opt {
                                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::Frame(close)}).await.unwrap();
                                            return
                                        } else {
                                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::Empty}).await.unwrap();
                                            return
                                        }
                                    },
                                    _ => todo!("Sent unsupported WS message!")
                                }
                            }
                            Err(err) => {
                                self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::Err(err)}).await.unwrap();
                                return
                            }
                        }
                    } else {
                        println!("There are no client packages left, returning.");
                        self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::Empty}).await.unwrap();
                        return
                    }
                }
                    send = self.from_server.recv() => {
                    if let Some(m) = send {
                        if let Err(err) = self.websocket.send(WsMessage::Text(m)).await {
                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::Err(err)}).await.unwrap();
                        };
                    } else {
                        println!("Server closed the sender, closing connection!");
                        match self.websocket.close().await {
                            Ok(()) => (),
                            Err(err) => {
                                if err.to_string() != "Trying to work with closed connection"{
                                    println!("Websocket error! {}", err);
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
