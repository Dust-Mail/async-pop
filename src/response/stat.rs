#[derive(Debug)]
pub struct StatResponse {
    message_count: usize,
    size: usize,
}

impl From<(usize, usize)> for StatResponse {
    fn from((count, size): (usize, usize)) -> Self {
        Self::new(count, size)
    }
}

impl StatResponse {
    pub fn new(message_count: usize, size: usize) -> Self {
        Self {
            message_count,
            size,
        }
    }

    pub fn counter(&self) -> usize {
        self.message_count
    }

    pub fn size(&self) -> usize {
        self.size
    }
}
