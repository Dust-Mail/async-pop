use super::types::{message::Text, number::Number};

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
    message: Option<Text>,
    items: Vec<UniqueId>,
}

impl Uidl {
    pub fn new<M: Into<Text>>(message: Option<M>, items: Vec<UniqueId>) -> Self {
        Self {
            message: message.map(|msg| msg.into()),
            items,
        }
    }

    pub fn items(&self) -> &[UniqueId] {
        self.items.as_ref()
    }

    pub fn message(&self) -> Option<&Text> {
        self.message.as_ref()
    }
}

#[derive(Debug)]
pub struct UniqueId {
    index: Number,
    id: Text,
}

impl UniqueId {
    pub fn new<I: Into<Number>, D: Into<Text>>(index: I, id: D) -> Self {
        Self {
            index: index.into(),
            id: id.into(),
        }
    }

    pub fn index(&self) -> &Number {
        &self.index
    }

    pub fn id(&self) -> &Text {
        &self.id
    }
}
