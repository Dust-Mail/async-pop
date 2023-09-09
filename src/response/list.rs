use super::{stat::StatResponse, types::number::Number};

#[derive(Debug)]
pub enum ListResponse {
    Multiple(List),
    Single(StatResponse),
}

impl From<List> for ListResponse {
    fn from(list: List) -> Self {
        Self::Multiple(list)
    }
}

impl From<StatResponse> for ListResponse {
    fn from(item: StatResponse) -> Self {
        Self::Single(item)
    }
}

#[derive(Debug)]
pub struct List {
    stats: StatResponse,
    items: Vec<ListItem>,
}

impl List {
    pub fn new(stats: StatResponse, items: Vec<ListItem>) -> Self {
        Self { stats, items }
    }

    pub fn items(&self) -> &[ListItem] {
        self.items.as_ref()
    }

    pub fn stats(&self) -> &StatResponse {
        &self.stats
    }
}

#[derive(Debug)]
pub struct ListItem {
    index: Number,
    size: Number,
}

impl ListItem {
    pub fn new<I: Into<Number>, S: Into<Number>>(index: I, size: S) -> Self {
        Self {
            index: index.into(),
            size: size.into(),
        }
    }

    pub fn index(&self) -> &Number {
        &self.index
    }

    pub fn size(&self) -> &Number {
        &self.size
    }
}
