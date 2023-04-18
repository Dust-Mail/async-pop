use crate::{
    constants::{LF, SPACE},
    types::Result,
};

use super::{Error, ErrorKind};

pub struct Stats(u32, u64);

pub enum StatsResponse {
    Stats(Stats),
    StatsList(Vec<Stats>),
}

impl StatsResponse {
    pub fn is_list(&self) -> bool {
        match self {
            StatsResponse::Stats(_) => false,
            StatsResponse::StatsList(_) => true,
        }
    }
}

impl Stats {
    pub fn messsage_count(&self) -> &u32 {
        &self.0
    }

    pub fn drop_size(&self) -> &u64 {
        &self.1
    }

    pub fn into_message_count(self) -> u32 {
        self.0
    }

    pub fn into_drop_size(self) -> u64 {
        self.1
    }

    pub fn from_response<S: AsRef<str>>(response: S) -> Result<StatsResponse> {
        let response = response.as_ref();
        let end_of_line = char::from_u32(LF as u32).unwrap();

        let split = response
            .split(end_of_line)
            .filter(|item| item.trim().len() != 0);

        let mut results: Vec<Stats> = Vec::new();

        for unparsed in split {
            let unparsed = unparsed.trim().to_ascii_lowercase();

            let mut count_and_id = unparsed.split(SPACE);

            let count = count_and_id.next().ok_or(Error::new(
                ErrorKind::InvalidResponse,
                "Missing message count from stats response",
            ))?;

            let count = count.parse::<u32>()?;

            let drop = count_and_id.next().ok_or(Error::new(
                ErrorKind::InvalidResponse,
                "Missing drop size from stats response",
            ))?;

            let drop = drop.parse::<u64>()?;

            results.push(Stats(count, drop));
        }

        if results.len() == 1 {
            let result = std::mem::take(&mut results[0]);

            Ok(StatsResponse::Stats(result))
        } else {
            Ok(StatsResponse::StatsList(results))
        }
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self(0, 0)
    }
}

#[cfg(test)]
mod test {
    use super::Stats;

    #[test]
    fn test_parse_unique_id() {
        let to_parse = "21 2123";

        let result = Stats::from_response(to_parse);

        assert!(result.is_ok());

        let to_parse = "\r\n\r    21 24325\n";

        let result = Stats::from_response(to_parse);

        assert!(!result.unwrap().is_list());

        let to_parse = "21 23123\n\r42 543543";

        let result = Stats::from_response(to_parse);

        assert!(result.unwrap().is_list());
    }
}
