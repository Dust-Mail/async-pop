macro_rules! collection {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {{
        core::convert::From::from([$(($k.into(), $v),)*])
    }};
    // set-like
    ($($v:expr),* $(,)?) => {{
        core::convert::From::from([$($v,)*])
    }};
}

pub(crate) use collection;

macro_rules! escape_newlines {
    ($input:expr) => {
        $input.replace("\n", "\\n").replace("\r", "\\r")
    };
}

pub(crate) use escape_newlines;
