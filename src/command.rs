use std::{collections::HashMap, fmt::Display, str::FromStr};

use crate::{
    error::{Error, ErrorKind},
    macros::collection,
};

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Noop,
    Uidl,
    Top,
    Dele,
    Rset,
    Retr,
    List,
    Stat,
    Apop,
    Auth,
    User,
    Pass,
    Quit,
    Capa,
    Greet,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (key, value) in Self::definitions().into_iter() {
            if &value == self {
                write!(f, "{}", key.to_ascii_uppercase())?;
                return Ok(());
            }
        }

        Ok(())
    }
}

impl Command {
    fn definitions() -> HashMap<String, Self> {
        use Command::*;

        collection!(
            "noop" => Noop,
            "uidl" => Uidl,
            "top" => Top,
            "dele" => Dele,
            "rset" => Rset,
            "retr" => Retr,
            "list" => List,
            "stat" => Stat,
            "apop" => Apop,
            "auth" => Auth,
            "user" => User,
            "quit" => Quit,
            "capa" => Capa,
            "pass" => Pass
        )
    }
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let to_match = s.to_lowercase();

        match Self::definitions().remove(&to_match) {
            Some(command) => Ok(command),
            None => Err(Error::new(
                ErrorKind::ParseCommand,
                format!("Could not recognize '{}' as a valid POP command", to_match),
            )),
        }
    }
}
