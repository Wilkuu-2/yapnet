use uuid::Uuid; 
use crate::protocol::Perm;

pub struct User {
    pub online: bool,
    pub uuid: Uuid,
}

pub struct Chat {
    pub perms: Vec<Perm>
}

impl Chat {
    pub fn can_write(&self, _: &User) -> bool {
        // Todo: Check permissions
        return true;
    }
    pub fn can_read(&self, _: &User) -> bool {
        // Todo: Check permissions
        return true;
    }
}
