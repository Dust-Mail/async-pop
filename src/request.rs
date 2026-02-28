use std::{fmt::Display, str::FromStr};

use crate::{command::Command, error::Error};

#[derive(Debug)]
pub struct Request {
    command: Command,
    args: Vec<String>,
}

impl From<Request> for Command {
    fn from(val: Request) -> Self {
        val.command
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
    pub fn new<A: Display>(command: Command, args: &[A]) -> Self {
        Self {
            command,
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
