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
