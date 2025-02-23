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

use serde::{Deserialize, Serialize};
pub mod body;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatSetup {
    pub name: String,
    pub perm: Perms,
}

pub type UserId = String;
pub type ChatId = String;
pub type RoleId = String;

pub trait MessageDataV2 {
    /// Returns the msg_type field
    fn msg_type(&self) -> &'static str;
    fn is_global(&self) -> bool;
    fn subject(&self) -> Option<UserId>;
    fn object(&self) -> Option<UserId>;
    fn chat(&self) -> Option<ChatId>;
}

/// Permissions for chats
///
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Perm {
    /// This user has permission
    #[serde(rename = "user")]
    User {
        /// 1: read, 2: write, 3: all
        rw: u8,
        /// Name of the user
        name: String,
    },
    /// This group has permission
    #[serde(rename = "group")]
    Group {
        /// 1: read, 2: write, 3: all
        rw: u8,
        /// Name of the group
        name: String,
    },
    /// Everyone has permission
    #[serde(rename = "any")]
    Any {
        /// 1: read, 2: write, 3: all
        rw: u8,
    },
}

impl Perm {
    pub fn check_player(&self, username: &String) -> Option<u8> {
        match self {
            Self::Any { rw } => Some(*rw),
            Self::User { name, rw } => {
                if *name == *username {
                    Some(*rw)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub fn check_group(&self, groupname: &String) -> Option<u8> {
        match self {
            Self::Any { rw } => Some(*rw),
            Self::Group { name, rw } => {
                if *name == *groupname {
                    Some(*rw)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Perms(Vec<Perm>);

impl Default for Perms {
    fn default() -> Self {
        Self::new()
    }
}

impl Perms {
    pub fn new() -> Self {
        Self(Vec::with_capacity(4))
    }

    pub fn wrap_vec(v: Vec<Perm>) -> Self {
        Self(v)
    }

    pub fn check_player(&self, username: &String) -> u8 {
        let mut rw: u8 = 0;
        for item in self.0.iter() {
            if let Some(p) = item.check_player(username) {
                rw |= p;
            }
        }
        rw
    }
    pub fn check_group(&self, groupname: &String) -> u8 {
        let mut rw: u8 = 0;
        for item in self.0.iter() {
            if let Some(p) = item.check_group(groupname) {
                rw |= p;
            }
        }
        rw
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageV2 {
    #[serde(default)]
    pub seq: u64,
    #[serde(flatten)]
    pub data: body::MessageV2Enum,
}
