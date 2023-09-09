use super::types::{message::Message, number::Number};

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
    message: Message,
    items: Vec<UniqueId>,
}

impl Uidl {
    pub fn new<M: Into<Message>>(message: M, items: Vec<UniqueId>) -> Self {
        Self {
            message: message.into(),
            items,
        }
    }

    pub fn items(&self) -> &[UniqueId] {
        self.items.as_ref()
    }

    pub fn message(&self) -> &Message {
        &self.message
    }
}

#[derive(Debug)]
pub struct UniqueId {
    index: Number,
    id: Message,
}

impl UniqueId {
    pub fn new<I: Into<Number>>(index: I, id: Message) -> Self {
        Self {
            index: index.into(),
            id,
        }
    }

    pub fn index(&self) -> &Number {
        &self.index
    }

    pub fn id(&self) -> &Message {
        &self.id
    }
}
