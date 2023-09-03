use super::stat::StatResponse;

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
    message: Option<String>,
    items: Vec<ListItem>,
}

impl List {
    pub fn new<M: Into<String>>(message: Option<M>, items: Vec<ListItem>) -> Self {
        Self {
            message: message.map(|msg| msg.into()),
            items,
        }
    }

    pub fn items(&self) -> &[ListItem] {
        self.items.as_ref()
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }
}

#[derive(Debug)]
pub struct ListItem {
    index: usize,
    size: usize,
}

impl ListItem {
    pub fn new(index: usize, size: usize) -> Self {
        Self { index, size }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn size(&self) -> usize {
        self.size
    }
}
