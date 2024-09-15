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
//

use std::collections::HashMap;

use uuid::Uuid; 

pub type Users = HashMap<String, User>;

pub struct User {
    pub online: bool,
    pub uuid: Uuid,
}

impl User {
    pub fn new(uuid: Uuid) -> Self {
        return Self {
            uuid, 
            online: true,
        }
    } 
} 
