use crate::types::Message;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Folder {
    pub path: Vec<String>,
}

impl From<&async_imap::types::Name> for Folder {
    fn from(value: &async_imap::types::Name) -> Self {
        Folder {
            path: match value.delimiter() {
                None => vec![value.name().to_string()],
                Some(d) => value.name().split(d).map(|s| s.to_string()).collect(),
            },
        }
    }
}

impl std::fmt::Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.join("/"))
    }
}

impl Folder {
    pub fn list_messages(&self) -> Vec<Message> {
        // TODO
        vec![Message { id: 1 }, Message { id: 2 }]
    }
}
