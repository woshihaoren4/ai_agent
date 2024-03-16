use crate::{CallBack, CallBackSet};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct DefaultCallbackSet {
    set: Mutex<HashMap<String, CallBack>>,
}

impl CallBackSet for DefaultCallbackSet {
    fn push(&self, code: String, cb: CallBack) {
        let mut lock = self.set.lock().unwrap();
        lock.insert(code, cb);
    }

    fn remove(&self, code: &str) -> Option<CallBack> {
        let mut lock = self.set.lock().unwrap();
        lock.remove(code)
    }
}
