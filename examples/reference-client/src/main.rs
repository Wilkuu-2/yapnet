use core::error;
use std::io;

use app::UIMessage;
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
            false => Some(rx.resubscribe())
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
