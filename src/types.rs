use crate::actions;

pub type Flags = std::collections::HashSet<String>;

pub struct Message {
    pub id: u32,
}

pub use crate::folder::Folder;

pub struct Context {}

pub type FilterSpec<'a> = Vec<(Folder, Vec<actions::Action>)>;
