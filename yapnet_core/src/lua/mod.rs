pub mod yapi;

use mlua::prelude::*; 

pub fn parse_table_field<'t, T: FromLua<'t>>(table: LuaTable<'t>, name: &'t str, default: T) -> T
{
   match table.get(name) {
        Ok(a) => a,
        Err(e) => { 
            eprintln!("Missing '{name}' field: {}", e);
            default
        },
    }
}
