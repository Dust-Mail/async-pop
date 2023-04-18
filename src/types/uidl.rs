use crate::{
    constants::{LF, SPACE},
    types::Result,
};

use super::{Error, ErrorKind};

pub struct UniqueID(u32, String);

pub enum UniqueIDResponse {
    UniqueID(UniqueID),
    UniqueIDList(Vec<UniqueID>),
}

impl UniqueIDResponse {
    pub fn is_list(&self) -> bool {
        match self {
            UniqueIDResponse::UniqueID(_) => false,
            UniqueIDResponse::UniqueIDList(_) => true,
        }
    }
}

impl UniqueID {
    pub fn unique_id(&self) -> &str {
        &self.1
    }

    pub fn into_unique_id(self) -> String {
        self.1
    }

    pub fn count(&self) -> &u32 {
        &self.0
    }

    pub fn into_count(self) -> u32 {
        self.0
    }

    pub fn from_response<S: AsRef<str>>(response: S) -> Result<UniqueIDResponse> {
        let response = response.as_ref();
        let end_of_line = char::from_u32(LF as u32).unwrap();

        let split = response
            .split(end_of_line)
            .filter(|item| item.trim().len() != 0);

        let mut results: Vec<UniqueID> = Vec::new();

        for unparsed in split {
            let unparsed = unparsed.trim().to_ascii_lowercase();

            let mut count_and_id = unparsed.split(SPACE);

            let count = count_and_id.next().ok_or(Error::new(
                ErrorKind::InvalidResponse,
                "Missing count from uidl response",
            ))?;

            let count = count.parse::<u32>()?;

            let unique_id = count_and_id.next().ok_or(Error::new(
                ErrorKind::InvalidResponse,
                "Missing unique id from uidl response",
            ))?;

            results.push(UniqueID(count, unique_id.to_string()));
        }

        if results.len() == 1 {
            let result = std::mem::take(&mut results[0]);

            Ok(UniqueIDResponse::UniqueID(result))
        } else {
            Ok(UniqueIDResponse::UniqueIDList(results))
        }
    }
}

impl Default for UniqueID {
    fn default() -> Self {
        Self(0, String::new())
    }
}

#[cfg(test)]
mod test {
    use super::UniqueID;

    #[test]
    fn test_parse_unique_id() {
        let to_parse = "21 super_unique";

        let result = UniqueID::from_response(to_parse);

        assert!(result.is_ok());

        let to_parse = "\r\n\r    21 super_unique\n";

        let result = UniqueID::from_response(to_parse);

        assert!(!result.unwrap().is_list());

        let to_parse = "21 super_unique\n\r42 yeah";

        let result = UniqueID::from_response(to_parse);

        assert!(result.unwrap().is_list());
    }
}
