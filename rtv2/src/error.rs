use std::error::Error;
use std::fmt::{Display, Formatter};
use wd_tools::PFErr;

#[derive(Debug)]
pub enum RTError{
    ContextStatusAbnormal(String),
    RuntimeDisable,
    UnknownNodeId(String),
    FlowLastNodeNil,
}
impl RTError{
    pub fn anyhow<T>(self)->anyhow::Result<T>{
        anyhow::Error::from(self).err()
    }
}
// impl From<RTError> for anyhow::Error{
//     fn from(value: RTError) -> Self {
//         anyhow::Error::from(value)
//     }
// }


impl Display for RTError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RTError::RuntimeDisable => {
                write!(f,"runtime disable")
            }
            RTError::UnknownNodeId(id)=>{
                write!(f,"unknown node id[{}]",id)
            }
            RTError::FlowLastNodeNil=>{
                write!(f,"flow next illegality")
            }
            RTError::ContextStatusAbnormal(s)=>{
                write!(f,"ctx status abnormal:{}",s)
            }
        }
    }
}

impl Error for RTError{}