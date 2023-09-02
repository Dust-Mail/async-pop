use std::{fmt::Display, str::FromStr};

use crate::{command::Command, error::Error};

pub struct Request {
    command: Command,
    args: Vec<String>,
}

impl Into<Command> for Request {
    fn into(self) -> Command {
        self.command
    }
}

impl From<Command> for Request {
    fn from(command: Command) -> Self {
        Self::new(command, &Vec::<String>::new())
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command)?;

        for arg in &self.args {
            write!(f, " {}", arg)?;
        }

        Ok(())
    }
}

impl FromStr for Request {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let command: Command = s.parse()?;

        Ok(command.into())
    }
}

impl Request {
    pub fn new<A: Display>(command: Command, args: &Vec<A>) -> Self {
        Self {
            command: command.into(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
        }
    }

    pub fn add_arg<A: Display>(&mut self, arg: A) {
        self.args.push(arg.to_string())
    }

    pub fn command(&self) -> &Command {
        &self.command
    }
}
