use std::path::PathBuf;
use mlua::prelude::*;
use mlua::StdLib;
use mlua::chunk;
use crate::game::User; 
use crate::protocol::Perm;

type RoleID = String; 
type ActionID = String; 



pub fn init_lua_from_argv() -> Lua {
    let mut args = std::env::args();
    let _ = args.next();
    let file_name = args.next().expect("No file argument given"); 
    init_lua(file_name.try_into().unwrap())
} 

pub fn init_lua<'table>(path: PathBuf) -> Lua {
    let opts = LuaOptions::new(); 
    let lua = Lua::new_with(StdLib::ALL_SAFE, opts).expect("Creation of lua runtime failed"); 
    
    push_api(&lua);

    {
        let file = path.to_str().unwrap();
        println!("Loading file: {}", file);
        lua.load(chunk!( 
            __game = dofile($file)
        )).exec().unwrap(); 
    }


    lua
}  

#[derive(Clone)]
pub struct LuaPlayer { username: String, role: RoleID, groups: Vec<Perm>, current_action: ActionID}

impl From<(&String,&User)> for LuaPlayer {
    fn from(value: (&String,&User)) -> Self {
        Self { username: value.0.clone(), role: "".to_string(), groups: vec![], current_action: "".to_string()}  
    }
} 

impl LuaUserData for LuaPlayer{ } 

fn yn_api_test(_lua: &Lua, arg: String) -> LuaResult<()> {
    println!("Hello {}", arg);
    Ok(())
} 

macro_rules! push_fns {
    ($lua:ident, $table:ident; $($fn:ident),*) => (
        $(
        $table.set(stringify!($fn), $lua.create_function($fn).expect(format!("Function  {} not lua-compatible", stringify!($fn)).as_str())).expect("Pushing onto api table failed");
        ),* 
    )
}


pub(super) fn push_api(l: &Lua) {
   let yn_api_table = l.create_table().unwrap();
   

    push_fns!(l, yn_api_table; 
        yn_api_test
    ); 

   // Push the final table into 
   l.globals().set("yapi", yn_api_table).unwrap();
}  
