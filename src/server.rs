use std::collections::HashMap;
use crate::{Message, MessageData};
use tokio::{select, 
    sync::mpsc::{channel, Receiver, Sender}, task::JoinHandle
}; 
use axum::extract::ws::{CloseFrame, Message as WsMessage, WebSocket};


/// Server stored handle to the client task 
pub struct ClientConnection{ 
    pub id: usize,
    pub to_client: Sender<super::Message>, 
    client_handle: JoinHandle<()>
} 

/// The client task and its state
pub struct Client{
    cid: usize, 
    from_server: Receiver<super::Message>, 
    to_server: Sender<super::Message>, 
    remove_clients: Sender<CloseConnection>,
    websocket: WebSocket,
} 

/// Client-task message to the server to close the channel and end the task
pub struct CloseConnection {
    pub id: usize,
    pub reason: CloseConnectionReason
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
pub struct ServerHandle  {
   pub messages: Sender<super::Message>,
   pub add_clients: Sender<WebSocket>,
   pub remove_clients: Sender<CloseConnection>,
} 

/// Websocket server task
pub struct Server  {
  pub messages:  Receiver<super::Message>,
  pub add_clients: Receiver<WebSocket>,
  pub remove_clients: Receiver<CloseConnection>,
  handle: ServerHandle,
  clients: HashMap<usize, ClientConnection>,
  users_connections: HashMap<usize, uuid::Uuid>,
} 

impl Server { 
    /// Make the server and the handle
    pub fn create() -> (Self, ServerHandle){
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
            handle: sh.clone(),
        };

        return (sv, sh)
    }

    /// The task that manages websocket connection
    pub async fn run(mut self) {
        loop {
            select! {
                client_opt = self.add_clients.recv() => {
                    let comm = client_opt.unwrap();
                    let id = self.clients.len(); 

                    let client = ClientConnection::create_client(id,comm,&self.handle); 
                    self.clients.insert(id, client); 
                }
                client_opt = self.remove_clients.recv() => {
                    let close = client_opt.unwrap();
                    self.clients.remove(&close.id);
                } 

                msg_opt = self.messages.recv() => {
                    let msg = msg_opt.unwrap();
                    println!("Message recieved!: {:?}", msg)
                } 
            }
        } 
    } 

}


impl ClientConnection {
    /// Creates a client connection and spawns the task
    pub fn create_client(id: usize, ws: WebSocket , handle: &ServerHandle) -> Self {
        
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

impl Client{
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
                                            Ok(msg) => {self.to_server.send(msg).await.unwrap()},
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
                                        if let Some(close) = close_opt {
                                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseFrame(close)}).await.unwrap(); 
                                        } else {
                                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseEmpty}).await.unwrap(); 
                                        }
                                    },
                                    _ => todo!("Sent unsupported WS message!")
                                }
                            }
                            Err(err) => { 
                                self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseErr(err)}).await.unwrap(); 
                            }
                        }
                    } else {
                        println!("Client closed the connection?!");
                        self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseEmpty}).await.unwrap(); 
                    }
                }
                    send = self.from_server.recv() => {
                    if let Some(m) = send { 
                        if let Err(err) = self.websocket.send(WsMessage::Text(serde_json::to_string(&m).expect("Serializing the message should never fail"))).await {
                            self.remove_clients.send(CloseConnection{id: self.cid, reason: CloseConnectionReason::ClientCloseErr(err)}).await.unwrap(); 
                        };
                    } else {
                        println!("Server closed the sender, closing connection!");
                        match self.websocket.close().await {
                            Ok(()) => (), 
                            Err(err) => println!("Websocket error! {:?}", err)
                        }; 
                        return;
                    }
                }
            }
        }
    } 
}
