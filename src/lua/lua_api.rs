use mlua::prelude::*;

use crate::{protocol::Perm, state::User};


type RoleID = String; 
type ActionID = String; 

#[derive(Clone)]
pub struct LuaPlayer { username: String, role: RoleID, groups: Vec<Perm>, current_action: ActionID}

impl From<(&String,&User<'_>)> for LuaPlayer {
    fn from(value: (&String,&User<'_>)) -> Self {
        Self { username: value.0.clone(), role: "".to_string(), groups: vec![], current_action: "".to_string()}  
    }
} 

impl LuaUserData for LuaPlayer{ } 

struct Ctx {
} 

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


