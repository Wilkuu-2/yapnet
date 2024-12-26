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

use std::{
    error,
    fmt::{self, format, Display},
    iter::repeat_n,
    str::FromStr,
};

use ratatui::text::Text;
use tokio::sync::broadcast::{Receiver, Sender};
use tui_textarea::TextArea;
use tui_widgets::scrollview::ScrollViewState;
use uuid::Uuid;
use yapnet_client::{Client, ClientAction};
use yapnet_core::prelude::{ChatSent, MessageData};

use crate::ui::SubListState;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

pub enum AppState {
    Login,
    Chat,
}

/// Application.
pub struct App<'a> {
    /// Is the application running?
    pub running: bool,
    pub on_connect: Option<Sender<()>>,
    pub messages: Vec<UIMessage>,
    pub client: Option<Client>,
    pub state: AppState,
    pub input: TextArea<'a>,
    pub scroll: SubListState,
    pub current_chat: String,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        let msg = vec![];
        Self {
            running: true,
            on_connect: None,
            current_chat: "NULL".to_string(),
            messages: msg,
            client: None,
            state: AppState::Login,
            input: Self::make_input(),
            scroll: SubListState::default(),
        }
    }
}

impl<'a> App<'a> {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    fn make_input() -> TextArea<'a> {
        TextArea::default()
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn submit_uimessage(&mut self, msg: UIMessage) {
        self.messages.push(msg)
    }

    pub fn get_username(&'a self) -> Option<String> {
        self.client
            .as_ref()
            .map_or(None, |x| x.state.username.as_ref().map(|z| z.clone()))
    }

    pub async fn enter_message(&mut self) {
        let content = self.input.lines().join("\n");
        self.input = Self::make_input();
        let command = AppCommand::from_str(&content);

        if let Ok(cmd) = command {
            let username_string = self.get_username();
            let username = username_string.as_ref().map(|f| f.as_str());
            self.submit_uimessage(UIMessage::cmd(username, content.as_str()));
            self.handle_command(cmd).await;
        } else {
            self.send_chat(content).await;
        }
    }

    pub async fn send_chat(&mut self, content: String) {
        if let Some(client) = &mut self.client {
            let res = client
                .send_message(yapnet_core::prelude::ChatSend {
                    chat_target: self.current_chat.clone(),
                    chat_content: content,
                }.into())
                .await;

            match res {
                Ok(()) => {}
                Err(e) => self.submit_uimessage(UIMessage::err(&format!("Client: {:?}", e))),
            }
        } else {
            self.submit_uimessage(UIMessage::sys("Not connected"))
        }
    }

    async fn handle_command(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::Connect(url) => {
                let sender = self.on_connect.as_ref().unwrap();
                let res = self.connect_socket(url, sender.clone()).await;
                match res {
                    Ok(()) => {}
                    Err(e) => self.submit_uimessage(UIMessage::err(&format!("Client: {:?}", e))),
                }
            }
            AppCommand::Register(player) => {
                if let Some(client) = &mut self.client {
                    client.send_register(player).await;
                } else {
                    self.submit_uimessage(UIMessage::sys("Not connected"));
                }
            }
            AppCommand::Login(uuid) => {
                if let Some(client) = &mut self.client {
                    client.send_login(uuid).await;
                } else {
                    self.submit_uimessage(UIMessage::sys("Not connected"));
                }
            }
            AppCommand::Chat => {
                if let Some(client) = &self.client {
                    // TODO: MOVE TO CLIENT
                    let list = client
                        .lobby
                        .chats
                        .keys()
                        .map(|c| format!("> {}", c))
                        .collect::<Vec<String>>()
                        .join("\n");
                    self.submit_uimessage(UIMessage::sys(&format!(
                        "Currently Open Chats:\n{}",
                        list
                    )))
                } else {
                    self.submit_uimessage(UIMessage::sys("Not connected"))
                }
            }
            AppCommand::Help => self.submit_uimessage(UIMessage::sys(HELP_MSG)),
            AppCommand::PlayerList => {
                if let Some(client) = &self.client {
                    // TODO: MOVE TO CLIENT
                    let list = client
                        .lobby
                        .players
                        .iter()
                        .map(|(n, p)| {
                            format!(
                                "> {}({}), {}",
                                n,
                                p.role,
                                match p.connected {
                                    true => "online",
                                    false => "offline",
                                }
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n");
                    self.submit_uimessage(UIMessage::sys(&format!(
                        "Currently Open Chats:\n{}",
                        list
                    )))
                } else {
                    self.submit_uimessage(UIMessage::sys("Not connected"))
                }
            }
            AppCommand::SwitchChat(chat) => {
                if let Some(client) = &self.client {
                    if client.lobby.chats.contains_key(&chat) {
                        self.current_chat = chat;
                    } else {
                        self.submit_uimessage(UIMessage::err(&format!("No chat {} found", &chat)))
                    }
                } else {
                    self.submit_uimessage(UIMessage::err("Not connected to server"))
                }
            }
            AppCommand::Error(err) => self.submit_uimessage(UIMessage::sys(&err)),
        }
    }

    pub async fn handle_wsmessage(&mut self, wait_on: Option<Receiver<()>>) {
        if let Some(mut wait) = wait_on {
            let _ = wait.recv().await;
        }
        // Assuming that the wait_on is set to the connect call.
        let res = self.client.as_mut().unwrap().recieve_and_handle().await;
        match res {
            Ok(r) => {
                self.handle_server(&r);
            }
            Err(e) => {
                self.submit_uimessage(UIMessage::err(&format!("Disconnected: {:?}", e)));
            }
        }
    }

    pub fn handle_server(&mut self, a: &ClientAction) {
        match a {
            ClientAction::Welcome => self.submit_uimessage(UIMessage::sys(&format!(
                "Successfully logged in!\n Username: {}\n Uuid {} \n Downloading recap...",
                self.get_username().unwrap(),
                self.client.as_ref().unwrap().state.token.unwrap()
            ))),
            ClientAction::RecapEnd => self.submit_uimessage(UIMessage::sys("Recap Done!")),
            ClientAction::PlayerJoined(username) => {
                self.submit_uimessage(UIMessage::sys(&format!("Player {} joined", username)));
            }
            ClientAction::PlayerLeft(username) => {
                self.submit_uimessage(UIMessage::sys(&format!("Player {} joined", username)));
            }
            ClientAction::Chat(e) => {
                let opt: &Result<ChatSent, ()> = &self
                    .client
                    .as_ref()
                    .unwrap()
                    .state
                    .messages
                    .get(e.clone())
                    .unwrap()
                    .data.clone().try_into(); 

                if let Ok(ChatSent{chat_sender,chat_target,chat_content}) = opt{

                    let a = UIMessage {
                        player: chat_sender.to_string(),
                        location: chat_target.to_string(),
                        content: chat_content.to_string(),
                    };

                    self.submit_uimessage(a);
                }
            }
            ClientAction::Error(e) => {
                self.submit_uimessage(UIMessage::err(&format!("Server Error: {}", e)));
            }
            ClientAction::Multiple(z) => {
                for x in z.iter() {
                    match x {
                        Ok(m) => self.handle_server(m),
                        Err(e) => {
                            self.submit_uimessage(UIMessage::err(&format!("Client: {:?}", e)));
                        }
                    }
                }
            }
            ClientAction::None => {}
        };
    }

    pub async fn connect_socket(
        &mut self,
        url: String,
        sender: Sender<()>,
    ) -> Result<(), yapnet_client::Error> {
        self.client = Some(Client::connect(url).await?);
        sender
            .send(())
            .expect("Message handler is only to be called once");
        Ok(())
    }
}

#[derive(Debug)]
pub struct UIMessage {
    location: String,
    player: String,
    content: String,
}

impl<'t> Display for &'t UIMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}: {}", self.player, self.location, self.content)
    }
}
impl UIMessage {
    pub fn line_count(&self, width: u16) -> u16 {
        let mut lines = 0u16;
        let mut liter = self.content.lines().map(|x| x.len());
        let mut begin: i32 = (liter.next().unwrap() + 3 + self.location.len() + self.player.len())
            .try_into()
            .unwrap();

        while begin > 0 {
            begin -= width as i32;
            lines += 1;
        }

        for l in liter {
            let mut a: i32 = l
                .try_into()
                .expect("The length of the line should be smaller that 32k characters");
            while a > 0 {
                a -= width as i32;
                lines += 1;
            }
        }
        lines
    }

    fn sys(content: &str) -> Self {
        Self {
            location: "NULL".to_string(),
            player: "SYSTEM".to_string(),
            content: content.to_string(),
        }
    }
    fn err(content: &str) -> Self {
        Self {
            location: "ERROR".to_string(),
            player: "SYSTEM".to_string(),
            content: content.to_string(),
        }
    }
    fn cmd(user: Option<&str>, content: &str) -> Self {
        let player = user.unwrap_or("UNKNOWN").to_string();
        Self {
            location: "NULL".to_string(),
            player,
            content: content.to_string(),
        }
    }
}

enum AppCommand {
    Connect(String),
    Register(String),
    Login(Uuid),
    PlayerList,
    SwitchChat(String),
    Chat,
    Help,
    Error(String),
}

impl FromStr for AppCommand {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().next().map_or(true, |c| c != '!') {
            return Err(());
        }

        let mut tokens = s[1..].split(" ");

        match tokens.next().unwrap_or("") {
            "connect" => Ok(tokens
                .next()
                .map_or(Self::Error("Missing argument: url".to_string()), |u| {
                    Self::Connect(u.to_string())
                })),
            "hello" => Ok(tokens
                .next()
                .map_or(Self::Error("Missing argument: username".to_string()), |u| {
                    Self::Register(u.to_string())
                })),
            "back" => {
                Ok(tokens
                    .next()
                    .map_or(Self::Error("Missing argument: uuid".to_string()), |u| {
                        Uuid::try_parse(u).map_or_else(
                            |e| Self::Error(format!("Invalid UUID: {:?}", e)),
                            |uuid| Self::Login(uuid),
                        )
                    }))
            }
            "list" => Ok(Self::PlayerList),
            "help" => Ok(Self::Help),
            "chat" => Ok(tokens
                .next()
                .map_or(Self::Chat, |u| Self::SwitchChat(u.to_string()))),
            _ => Ok(Self::Error("Unknown command".to_string())),
        }
    }
}

const HELP_MSG: &str = r###"
Commands: 
 - connect url    -> connect to a server
 - hello name     -> join the game with the name 
 - back uuid      -> return to the game using a uuid
 - list           -> list the players 
 - chat           -> list the chats
 - chat chat_name -> switch to this chat 
 - help           -> display this message
"###;
