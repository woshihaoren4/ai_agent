use std::path::Path;
use lazy_static::lazy_static;
use sled::{Db, IVec};
use wd_tools::PFOk;

lazy_static!{
    pub static ref UIB:UserInfoBank = UserInfoBank::new("./user");
}

pub struct UserInfoBank{
    db:Db
}

impl UserInfoBank{
    pub fn new<P:AsRef<Path>>(path:P)->Self{
        let db = sled::open(path).unwrap();
        Self{db}
    }


    pub fn get(&self,key:&str)->anyhow::Result<String>{
        let opt = self.db.get(key)?;
        match opt {
            None => String::new().ok(),
            Some(iv) => {
                String::from_utf8_lossy(iv.as_ref()).to_string().ok()
            }
        }
    }
    pub fn set<K,V>(&self,key:K,val:V)->anyhow::Result<()>
        where
            K: AsRef<[u8]>,
            V: Into<IVec>,
    {
        self.db.insert(key,val)?;
        Ok(())
    }
}

impl Drop for UserInfoBank {
    fn drop(&mut self) {
        if let Err(e) = self.db.flush(){
            wd_log::log_field("error",e).error("drop UserInfoBank, flush db error");
        }
    }
}

#[cfg(test)]
mod test{
    use crate::pkg::user_info_bank::UserInfoBank;

    #[test]
    fn test_db(){
        let uib = UserInfoBank::new("./user_info");
        uib.set("user1-hello","world").unwrap();
        let value = uib.get("user1-hello").unwrap();
        assert_eq!(value.as_str(),"world")
    }

}

