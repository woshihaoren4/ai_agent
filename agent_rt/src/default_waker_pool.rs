use crate::{WakerCallBack, WakerWaitPool};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct DefaultWakerPool {
    map: Mutex<HashMap<String, WakerCallBack>>,
}

impl WakerWaitPool for DefaultWakerPool {
    fn push(&self, code: String, waker: WakerCallBack) {
        let mut map = match self.map.lock() {
            Ok(o) => o,
            Err(e) => {
                wd_log::log_error_ln!("DefaultWakerPool,push error:{}", e);
                return;
            }
        };
        map.insert(code, waker);
    }

    fn remove(&self, code: &str) -> Option<WakerCallBack> {
        let mut map = match self.map.lock() {
            Ok(o) => o,
            Err(e) => {
                wd_log::log_error_ln!("DefaultWakerPool,remove error:{}", e);
                return None;
            }
        };
        map.remove(code)
    }
}
