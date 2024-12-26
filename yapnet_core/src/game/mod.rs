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

use crate::prelude::Message;

pub mod chat;
pub mod history;
pub mod user;

#[derive(Debug)]
pub enum MessageResult {
    /// Send message to everyone
    Broadcast(Message),
    /// Send message to everyone but the client who's message we are reacting too
    BroadcastExclusive(Message),
    /// Error, only send the message to the one client
    Error(Message),
    /// Only send the message to the one client who sent this message
    Return(Message),
    /// Composite message for things like joining, leaving and recap
    Many(Vec<MessageResult>),
    /// Bulk messages, like Recaps
    Bulk(Vec<Message>),
    /// Empty
    None,
}
