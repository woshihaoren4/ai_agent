use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum Error {
    NextNodeNull,
}

impl<T> Into<anyhow::Result<T>> for Error {
    fn into(self) -> anyhow::Result<T> {
        Err(anyhow::Error::from(self))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NextNodeNull => {
                write!(
                    f,
                    ">NextNodeNull< next node is null, service node can not call next function."
                )
            }
        }
    }
}

impl std::error::Error for Error {}
