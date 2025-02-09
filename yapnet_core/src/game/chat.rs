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

use super::user::User;
use crate::protocol::Perms;
use std::collections::HashMap;

pub type Chats = HashMap<String, Chat>;
pub type MessageRef = usize;

pub struct Chat {
    pub perms: Perms,
    pub messages: Vec<MessageRef>,
}

impl Chat {
    pub fn new(perms: Perms) -> Self {
        Self {
            perms,
            messages: vec![],
        }
    }
    pub fn can_write(&self, _: &User) -> bool {
        // Todo: Check permissions
        true
    }
    pub fn can_read(&self, _: &User) -> bool {
        // Todo: Check permissions
        true
    }
}
