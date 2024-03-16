use crate::Memory;
use async_openai::types::ChatCompletionRequestMessage;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::RwLock;
use wd_tools::{PFErr, PFOk};

#[derive(Default, Debug)]
pub struct SimpleMemory {
    list: RwLock<Vec<ChatCompletionRequestMessage>>,
}

impl Memory for SimpleMemory {
    fn load_context(&self, max: usize) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
        let read = self.list.read().unwrap();
        let len = read.len();
        if len < max {
            read.deref().clone().ok()
        } else {
            let mut list = Vec::with_capacity(max);
            for i in (len - max)..len {
                list.push(read.deref()[i].clone())
            }
            Ok(list)
        }
    }

    fn recall_user_tag(&self) -> anyhow::Result<HashMap<String, String>> {
        anyhow::anyhow!("todo").err()
    }

    fn add_session_log(&self, mut record: Vec<ChatCompletionRequestMessage>) {
        let mut write = self.list.write().unwrap();
        write.append(&mut record);
    }
}
