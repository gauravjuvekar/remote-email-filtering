use crate::types;
use crate::types::Message;

impl types::Folder {
    pub fn list_messages(&self) -> Vec<Message> {
        // TODO
        vec![Message { id: 1 }, Message { id: 2 }]
    }
}
