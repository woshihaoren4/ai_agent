use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum RTError{
    RuntimeDisable,
    UnknownNodeId(String),
    FlowLastNodeNil,
}

impl Into<anyhow::Error> for RTError{
    fn into(self) -> anyhow::Error {
        anyhow::Error::from(self)
    }
}

impl Display for RTError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RTError::RuntimeDisable => {
                write!(f,"runtime disable")
            }
            RTError::UnknownNodeId(id)=>{
                write!(f,"unknown node id[{}]",id)
            }
        }
    }
}

impl Error for RTError{}