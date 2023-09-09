use super::stat::Stat;

#[derive(Debug)]
pub enum ListResponse {
    Multiple(List),
    Single(Stat),
}

impl From<List> for ListResponse {
    fn from(list: List) -> Self {
        Self::Multiple(list)
    }
}

impl From<Stat> for ListResponse {
    fn from(item: Stat) -> Self {
        Self::Single(item)
    }
}

#[derive(Debug)]
pub struct List {
    stats: Option<Stat>,
    items: Vec<Stat>,
}

impl List {
    pub fn new(stats: Option<Stat>, items: Vec<Stat>) -> Self {
        Self { stats, items }
    }

    pub fn items(&self) -> &[Stat] {
        self.items.as_ref()
    }

    pub fn stats(&self) -> Option<&Stat> {
        self.stats.as_ref()
    }
}
