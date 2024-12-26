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

use std::io;

use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::broadcast;

use crate::{
    app::{App, AppResult},
    event::{Event, EventHandler},
    handler::handle_key_events,
    tui::Tui,
};

pub mod app;
pub mod event;
pub mod handler;
pub mod tui;
pub mod ui;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Create an application.
    let mut app = App::new();

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Todo: add ability to reconnect
    let (tx, rx) = broadcast::channel(10);
    app.on_connect = Some(tx);

    tui.draw(&mut app)?;
    // Start the main loop.
    while app.running {
        let wakey = match app.client.is_some() {
            true => None,
            false => Some(rx.resubscribe()),
        };

        tokio::select! {
            e = tui.events.next() => {
                let event = e?;
                // Render the user interface.
                // Handle events.

                match event {
                    Event::Tick => app.tick(),
                    Event::Key(key_event) => handle_key_events(key_event, &mut app).await?,
                    Event::Mouse(_) => {}
                    Event::Resize(_, _) => {}
                }
            }
            _ = app.handle_wsmessage(wakey)  => {
            }
        }
        tui.draw(&mut app)?;
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
