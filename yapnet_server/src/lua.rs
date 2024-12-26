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

use std::vec;

use crate::state::State;
use mlua::prelude::*;
use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc};
use yapnet_core::protocol::{MessageV2 as Message, Perms};
use yapnet_core::{game::chat::Chat, protocol::Perm};

// use mlua::LuaSerdeExt;

use yapnet_core::lua::yapi::LuaPlayer;

pub async fn init_lua() {}

pub fn state_init(lua: Lua) -> State {
    let mut state = State::new();
    {
        let globals = lua.globals();

        let game: LuaTable = globals
            .get("__game")
            .expect("Cannot find the configuration chunk ( return { .. } )");
        let chats: LuaTable = game.get("chats").expect("Cannot find chats!");
        for pair in chats.pairs::<String, LuaTable>() {
            match pair {
                Ok((name, table)) => {
                    let allowed_group: String =
                        yapnet_core::lua::parse_table_field(table, "allowed", "none".to_string());

                    let perm = match allowed_group.as_str() {
                        "any" | "all" => Perm::Any { rw: 3 },
                        "none" => Perm::User {
                            rw: 3,
                            name: String::from("__Noone"),
                        },
                        g => Perm::Group {
                            rw: 3,
                            name: g.to_string(),
                        },
                    };

                    let v = Chat {
                        perms: Perms::wrap_vec(vec![perm]),
                        messages: vec![],
                    };
                    state.chats.insert(name.clone(), v);
                }
                Err(e) => eprintln!("Cannot parse chat: {}", e),
            }
        }
    }

    state.lua_state = Some(LuaState { lua });
    state.push_setup_message();

    state
}

pub struct LuaState {
    pub lua: Lua,
}

pub struct StateFrame {
    players: HashMap<String, LuaPlayer>,
    pub outbound: Vec<Message>,
}

impl StateFrame {
    pub fn make(value: &State) -> Self {
        let mut players = HashMap::new();
        let outbound = vec![];

        for u in value.users.iter() {
            players.insert(u.0.clone(), u.into());
        }

        Self { players, outbound }
    }
}

impl LuaUserData for StateFrame {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        // Gets info on a player
        methods.add_method("get_player_info", |_, this, arg: String| {
            match this.players.get(&arg) {
                Some(pl) => Ok(Some(pl.clone())),
                None => Ok(None),
            }
        });

        // Sends a message
        methods.add_method_mut("send_message", |_, this, arg: LuaTable| {
            // Serde hack
            let msg: Message = serde_json::to_value(arg)
                .and_then(serde_json::from_value)
                .map_err(LuaError::external)?;
            this.outbound.push(msg);
            Ok(())
        });
    }
}

impl LuaState {
    #[inline]
    pub fn get_setup_table<'t, 'lua>(&'lua self) -> LuaTable<'t>
    where
        'lua: 't,
    {
        self.lua
            .globals()
            .get::<_, LuaTable<'t>>("__game")
            .expect("The __game table should always be there")
    }

    pub fn callback<'lua>(
        &'lua self,
        callback_name: &'static str,
        frame: Arc<Mutex<StateFrame>>,
        mut args: LuaMultiValue<'lua>,
    ) {
        match self.get_setup_table().get::<_, LuaFunction>(callback_name) {
            Ok(oc) => {
                let call_res = self.lua.scope(|scope| {
                    let frame_s = scope.create_userdata(frame.clone())?;
                    args.push_front(frame_s.into_lua(&self.lua)?);
                    oc.call(args)?; 
                    Ok(())

                });
                if let Err(err) = call_res {
                    eprintln!("Error in callback '{}'\n{}", callback_name, err)
                }
            }
            Err(e) => eprintln!("WARNING: {}", e),
        };
    }
}
