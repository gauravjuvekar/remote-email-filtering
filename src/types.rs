use crate::actions;

pub type Flags = std::collections::HashSet<String>;

pub struct Message {
    pub id: u32,
}

#[derive(Clone)]
pub struct Folder {
    pub path: Vec<String>,
}


pub struct Context {}

pub type FilterSpec<'a> = Vec<(Folder, Vec<actions::Action>)>;
