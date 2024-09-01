use std::vec;

use mlua::chunk;
use mlua::prelude::*; 
use mlua::StdLib;
use server_api::LuaState;
use crate::protocol::Perm;
use crate::state::{State,Chat}; 
mod lua_api;
pub mod server_api;

pub async fn init_lua<'table>() -> Lua {
    let opts = LuaOptions::new(); 
    let lua = Lua::new_with(StdLib::ALL_SAFE, opts).expect("Creation of lua runtime failed"); 
    
    lua_api::push_api(&lua);

    {
        let mut args = std::env::args();
        let _ = args.next();
        let file_name = args.next().expect("No file argument given"); 
        println!("Loading file: {}", file_name);
        // let file = tokio::fs::read_to_string(file_name).await.expect("File not found!");
        lua.load(chunk!( 
            __game = dofile($file_name)
        )).exec().unwrap(); 
    }


    lua
}  

fn parse_table_field<'t, T: FromLua<'t>>(table: LuaTable<'t>, name: &'t str, default: T) -> T
{
   match table.get(name) {
        Ok(a) => a,
        Err(e) => { 
            eprintln!("Missing '{name}' field: {}", e);
            default
        },
    }
} 

pub fn state_init<'server>(lua: Lua) -> State<'server> {
    let mut state = State::new();
    {
        let globals = lua.globals();

        let game: LuaTable = globals.get("__game").expect("Cannot find the configuration chunk ( return { .. } )");
        let chats: LuaTable = game.get("chats").expect("Cannot find chats!"); 
        for pair in chats.pairs::<String, LuaTable>() {
            match pair {
                Ok((name,table)) => {
                    let allowed_group: String = parse_table_field(table, "allowed", "none".to_string());

                    let perm = match allowed_group.as_str() {
                        "any" | "all" => Perm::Any { rw: 3 },
                        "none" => Perm::User { rw: 3, name: String::from("__Noone") },
                        g => Perm::Group { rw: 3, name: g.to_string() },
                    };
                    
                    let v = Chat {perms: vec![perm]}; 
                    state.chats.insert(name.clone(), v); 
                }, 
                Err(e) => eprintln!("Cannot parse chat: {}", e), 
            }
        } 
    }

    state.lua_state = Some(LuaState { lua });
    state.push_setup_message();

    state
    
}  
