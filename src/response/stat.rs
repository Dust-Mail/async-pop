use super::types::number::Number;

#[derive(Debug)]
pub struct StatResponse {
    message_count: Number,
    size: Number,
}

impl StatResponse {
    pub fn new<C: Into<Number>, S: Into<Number>>(message_count: C, size: S) -> Self {
        Self {
            message_count: message_count.into(),
            size: size.into(),
        }
    }

    pub fn counter(&self) -> &Number {
        &self.message_count
    }

    pub fn size(&self) -> &Number {
        &self.size
    }
}
