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
//
use std::{fmt::format, u16};

use ratatui::{
    buffer::Buffer, layout::{self, Alignment, Constraint, Layout, Rect}, style::{Color, Style}, symbols::{self, border::Set as BSet, scrollbar}, text::Text, widgets::{Block, BorderType, Borders, List, Scrollbar, ScrollbarState, StatefulWidget, Widget}, Frame
};

use crate::app::{App, UIMessage};

enum InputMode {Normal, Editing}


/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    
    let msg_border_set = BSet {
        top_left: symbols::line::THICK_VERTICAL_RIGHT,
        top_right: symbols::line::THICK_VERTICAL_LEFT,
        bottom_left: symbols::line::THICK_VERTICAL_RIGHT,
        bottom_right: symbols::line::THICK_VERTICAL_LEFT,
        ..symbols::border::PLAIN
    };
    let style = Style::default().fg(Color::LightMagenta).bg(Color::Reset);

    // Container Block
    let title = Block::bordered()
                    .title("Yapnet Reference Client. `Esc`, `Ctr-C`, `q` to exit`, !help for commands")
                    .title_alignment(Alignment::Center)
                    .border_type(BorderType::Double)
                    .style(style);
    
    // Lay out the window 
    let layout = Layout::default()
        .direction(layout::Direction::Vertical)
        .constraints(vec![Constraint::Fill(1), Constraint::Max(4)])
        .split(title.inner(frame.area()));
    
    // Create the view and border
    let messages_block = Block::default().borders(Borders::TOP | Borders::BOTTOM).border_type(BorderType::Plain).border_set(msg_border_set).style(style); 
    // Render chat list into the view
    let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight).style(style); 
    let sl = SubList{messages: &app.messages, scrollbar};

    
    // Render the input box
    frame.render_widget(&app.input, layout[1]);

    // Render the chat frame
    frame.render_stateful_widget(sl, messages_block.inner(layout[0]), &mut app.scroll);

    frame.render_widget(messages_block, layout[0]);

    // Render the big frame
    frame.render_widget(title, frame.area());
    

}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SubListState {
    head: usize,
    // TODO: List state for selection
} 

impl SubListState {
    pub fn scroll_down(&mut self) {
       self.head = self.head.saturating_add(1); 
    }
    pub fn scroll_up(&mut self) {
       self.head =  self.head.saturating_sub(1); 
    }
} 

pub struct SubList<'a, 'b> {
    messages: &'a [UIMessage],
    scrollbar: Scrollbar<'b>,
}

impl<'a, 'b> SubList<'a, 'b > {
}

impl<'a, 'b> StatefulWidget for SubList<'a, 'b> {
    type State = SubListState;

    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {
        let w = area.width - 1; // Size of the list
        let head = state.head.max(area.height.into());
        let mut h: usize = 0; 
        let mut top_found = false;
        let mut bottom_found = false; 
        let mut nb: usize = 0; 
        let mut ne: usize = 0; 
        let mut t = head.saturating_sub(area.height.into());
        let mut b = 0; 
        for a in self.messages.iter().enumerate() {
            h += a.1.line_count(area.width) as usize;
            if top_found {
                if h >= head {
                    ne = a.0 + 1; 
                    bottom_found = true; 
                    // See if statement above
                    b = unsafe { h.unchecked_sub(head) };
                    break;
              } 
            } else {
                if h >= t {
                    // Found the topmost thing
                    t = h - t;
                    nb = a.0;
                    top_found = true; 
                } 
            } 
        } 
        if !top_found {
            nb = 0; 
            ne = self.messages.len(); 
            t = 0; 
            b = 0;
        } else if !bottom_found {
            nb = t; 
            ne = self.messages.len(); 
            state.head = h;
        }

        assert!(nb <= ne);
        let mut scrollbar = ScrollbarState::new(self.messages.len()-(ne - nb)).position(nb).viewport_content_length(ne - nb);
        let new_area = Rect::new(0,0, w, area.height + t as u16 + b as u16); 
        let mut buffer = Buffer::empty(new_area); 
        
        let items: Vec<String> = self.messages[nb..ne].iter().map(|x| x.to_string()).collect();
        Widget::render(List::new(items), new_area, &mut buffer);
       
        // Render out from the temporary buffer
        for y in 0..area.height {
            let yi = y + t as u16; 
            let yb = y + area.y;
            for x in 0..w {
                let xb = x + area.x;
                let cell = buffer[(x,yi)].clone(); 
                buf[(xb, yb)].set_symbol(cell.symbol()).set_style(cell.style());
            }
        }



       // render scrollbar
       self.scrollbar.render(Rect::new(w,area.y,1,area.height), buf, &mut scrollbar); 
       // Text::styled(format!("[hd, t, b, nb, ne, len ,bottom, top_]\n[{}, {}, {}, {}, {}, {}, {}, {}]", head, t, b, nb, ne, self.messages.len(), bottom_found, top_found), Style::new().bg(Color::Cyan).fg(Color::Black)).render(Rect::new(area.x + 1,area.y,area.width,3), buf);
    }
} 
