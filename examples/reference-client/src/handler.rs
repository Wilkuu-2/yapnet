use crate::app::{App, AppResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::{Input};

/// Handles the key events and updates the state of [`App`].
pub async fn handle_key_events<'a>(key_event: KeyEvent, app: &mut App<'a>) -> AppResult<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
            return Ok(())
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
                return Ok(())
            }
        }
        KeyCode::Char('m') | KeyCode::Char('M')=> {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.enter_message().await;
                return Ok(());
            } 
        } 
        KeyCode::Enter => {
            app.enter_message().await; 
            return Ok(());
        } 

        KeyCode::Up => {
            if key_event.modifiers.intersects(KeyModifiers::SHIFT){
                app.scroll.scroll_up();
                return Ok(())
            }
        }
        KeyCode::Down => {
            if key_event.modifiers.intersects(KeyModifiers::SHIFT){
                app.scroll.scroll_down();
                return Ok(())
            }
        }

        _ => { }
    } 
    app.input.input(Input::from(key_event));
    Ok(())
}

