use crate::types;

pub struct ChangeFlags {
    set: types::Flags,
    clear: types::Flags,
}

pub trait LogicAction {
    fn process_msg(
        &self,
        msg: &types::Message,
        folder: &types::Folder,
    ) -> Vec<Action>;
}

pub enum Action {
    Logic(Box<dyn LogicAction>),
    Move(types::Folder),
    Flags(ChangeFlags),
    Stop,
}
