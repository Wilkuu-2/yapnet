use crate::state::{error_result, MessageResult, State};
use crate::{Message, MessageData};
use axum::extract::ws::{CloseFrame, Message as WsMessage, WebSocket};
use std::collections::HashMap;
use tokio::{
    select,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};

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
    to_server: Sender<super::Message>,
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
    pub messages: Sender<super::Message>,
    pub add_clients: Sender<WebSocket>,
    pub remove_clients: Sender<CloseConnection>,
}

/// Websocket server task
pub struct Server<'a> {
    pub messages: Receiver<super::Message>,
    pub add_clients: Receiver<WebSocket>,
    pub remove_clients: Receiver<CloseConnection>,
    state: State<'a>,
    handle: ServerHandle,
    clients: HashMap<usize, ClientConnection>,
    users_connections: HashMap<usize, String>,
    // TODO: Assign id's more efficiently, maybe also use uuid here, or
    // remake the `clients` into a array and manage that.
    highest_id: usize,
}

impl<'a> Server<'_> {
    /// Make the server and the handle
    pub fn create() -> (Self, ServerHandle) {
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
            state: State::new(),
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
            MessageData::Hello { username } => match self.state.new_user(&username) {
                Ok(o) => {
                    self.users_connections
                        .insert(cid as usize, username.clone());
                    o
                }
                Err(o) => o,
            },
            MessageData::Back { token } => match self.state.reauth_user(token) {
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

    async fn send_result(&self, cid: usize, m: MessageResult) {
        match m {
            MessageResult::Error(m) | MessageResult::Return(m) => {
                if let Some(client) = self.clients.get(&cid) {
                    client.to_client.send(m).await.unwrap() // TODO: Handle error
                }
            }
            MessageResult::BroadcastExclusive(m) => {
                for (cid2, client) in &self.clients {
                    if cid == *cid2 {
                        continue;
                    }
                    client.to_client.send(m.clone()).await.unwrap()
                    // TODO: Handle error and see what we can do about the clone
                }
            }
            MessageResult::Broadcast(m) => {
                for (_, client) in &self.clients {
                    client.to_client.send(m.clone()).await.unwrap()
                    // TODO: Handle error and see what we can do about the clone
                }
            }

            // Recursively send all the results
            MessageResult::Many(ms) => {
                for m in ms {
                    Box::pin(self.send_result(cid, m)).await
                }
            }

            MessageResult::None => (),
            MessageResult::Bulk(messages) => {
                let client = self.clients.get(&cid).unwrap();
                for m in messages {
                    client.to_client.send(m).await.unwrap();
                    // TODO: Handle error
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
                                                    data: MessageData::Error{
                                                        kind: "InvalidMessage".to_string(),
                                                        info: format!("{:?}", err),
                                                        details: String::new(),
                                                    }
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
