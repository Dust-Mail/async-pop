#[derive(Debug)]
pub enum UidlResponse {
    Multiple(Uidl),
    Single(UniqueId),
}

impl From<Uidl> for UidlResponse {
    fn from(value: Uidl) -> Self {
        Self::Multiple(value)
    }
}

impl From<UniqueId> for UidlResponse {
    fn from(value: UniqueId) -> Self {
        Self::Single(value)
    }
}

#[derive(Debug)]
pub struct Uidl {
    message: Option<String>,
    list: Vec<UniqueId>,
}

impl Uidl {
    pub fn new<M: Into<String>>(message: Option<M>, list: Vec<UniqueId>) -> Self {
        Self {
            message: message.map(|msg| msg.into()),
            list,
        }
    }

    pub fn list(&self) -> &[UniqueId] {
        self.list.as_ref()
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }
}

#[derive(Debug)]
pub struct UniqueId {
    index: usize,
    id: String,
}

impl UniqueId {
    pub fn new(index: usize, id: String) -> Self {
        Self { index, id }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
    }
}
