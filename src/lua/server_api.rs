use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc};

use mlua::prelude::*;
use mlua::LuaSerdeExt;

use crate::{state::State, Message};

use super::lua_api::LuaPlayer; 

pub struct LuaState {
    pub lua: Lua,
} 

pub struct StateFrame where 
{
    players: HashMap<String, LuaPlayer>,
    pub outbound: Vec<Message>, 
}

impl StateFrame where
{
   pub fn make(value: &State) -> Self {
        let mut players = HashMap::new(); 
        let outbound = vec![];

        for u in value.users.iter() {
            players.insert(u.0.clone(), u.into());
        }

        Self {players, outbound}
        
    }
}

impl LuaUserData for StateFrame where 
{
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
    }
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        // Gets info on a player
        methods.add_method("get_player_info", |_, this, arg: String| {
            match this.players.get(&arg){
                Some(pl) => Ok(Some(pl.clone())),
                None => Ok(None),
            }}); 

        // Sends a message 
        methods.add_method_mut("send_message", |_, this, arg: LuaTable| 
            {
                // Serde hack
                let msg: Message = serde_json::to_value(arg).and_then(serde_json::from_value).map_err(LuaError::external)?;
                this.outbound.push(msg); 
                Ok(())
            });
    }

} 


impl LuaState {

    #[inline]
    pub fn get_setup_table<'t,'lua>(&'lua self) -> LuaTable<'t> where 
        'lua: 't
    {
        self.lua.globals().get::<_, LuaTable<'t>>("__game").expect("The __game table should always be there")
    } 

    pub fn callback<'lua>(&'lua self, callback_name: &'static str, frame: Arc<Mutex<StateFrame>>, mut args: LuaMultiValue<'lua>)  
    {
        match self.get_setup_table().get::<_,LuaFunction>(callback_name) {
            Ok(oc) => {
                
                let call_res = self.lua.scope( |scope| { 
                    let frame_s = scope.create_userdata(frame.clone())?;
                    args.push_front(frame_s.into_lua(&self.lua)?);
                    oc.call(args)?;
                    Ok(())
                });
                if let Err(err) = call_res {
                    eprintln!("Error in callback '{}'\n{}", callback_name, err)
                }
            },
            Err(e) => eprintln!("WARNING: {}", e),
        };
    } 
} 


