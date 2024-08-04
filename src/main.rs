#![allow(dead_code)]

use std::net::SocketAddr;
use axum::{
    routing::get, Router,
    extract::{State, ws::{WebSocket, WebSocketUpgrade}},
    response::Response
}; 
use tower_http::services::ServeDir;
use tokio; 


mod protocol; 
mod server; 
mod state;

pub use protocol::MessageData;
pub use protocol::Message;


/// Axum state (Arc)
pub type AppState = std::sync::Arc<AppStateT>;
/// Inner type for the axum state 
pub struct AppStateT {
    pub server_handle: server::ServerHandle, 
}

/// Entry
#[tokio::main]
async fn main() {
    // TODO: Import tracing crate and figure out good logging strategy
    //tracing_subscriber::fmt::init(); 
    
    let (server, handle) = server::Server::create(); 

    let state = std::sync::Arc::new(
        AppStateT{
        server_handle: handle,
        }
    );

    let app: Router<()> = Router::new() 
        .route("/ws", get(handle_ws))
        .nest_service("/",ServeDir::new("client")) // Try finding files if it is not ws
        .with_state(state);

    
        
    let addr = SocketAddr::from(([127,0,0,1], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let axum_server = async {
        axum::serve(listener, app).await.unwrap()
    }; 

    tokio::join!(
        server.run(),
        axum_server,
    );
    
}


async fn handle_ws(ws: WebSocketUpgrade, State(state): State<AppState> ) -> Response {
    ws.on_upgrade(|socket| handle_client(socket, state))
} 


async fn handle_client(socket: WebSocket, state: AppState){
    let a = state.server_handle.add_clients.clone();
    a.send(socket).await.unwrap() 
}

#[macro_export]
macro_rules! handler {
    {$variant:pat = $var:expr => $code:block} => {
        if let $variant = $var { $code } 
        else {unreachable!("This should always match {}", stringify!($item))} 
    };
    {$var:expr;  $($variant:pat => $code:block),+} => {
        match $var {
           $($variant => $code)+
           _ => unreachable!("Exhausted handler patterns: {}", !stringify!($($variant)+))
        } 
    };
    
}
