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

pub mod message;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatSetup {
    pub name: String,
    pub perm: Vec<Perm> 
} 

/// Permissions for chats
/// 
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Perm {
    /// This user has permission
    #[serde(rename = "user")]
    User{
        /// 1: read, 2: write, 3: all 
        rw: u8,  
        /// Name of the user
        name: String
    }, 
    /// This group has permission
    #[serde(rename = "group")]
    Group{ 
        /// 1: read, 2: write, 3: all 
        rw: u8, 
        /// Name of the group
        name: String 
    }, 
    /// Everyone has permission 
    #[serde(rename = "any")]
    Any{
        /// 1: read, 2: write, 3: all 
        rw: u8,  
    },
}  
