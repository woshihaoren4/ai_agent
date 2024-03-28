use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use async_openai::Client;
use async_openai::types::{ChatCompletionMessageToolCall, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs, ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolType, CreateChatCompletionRequestArgs, FunctionObjectArgs};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wd_tools::{PFErr, PFOk};
use rt::{Context, Node, TaskInput, TaskOutput};
use crate::consts::{go_next_or_over, user_id_from_ctx};
use crate::Memory;
use crate::pkg::{DashVector, UIB};

#[derive(Debug,Default,Serialize,Deserialize)]
pub struct SummeryList{
    list:Vec<String>
}
#[derive(Debug,Default)]
pub struct ShortLongMemory{
    short_context:Vec<ChatCompletionRequestMessage>,
    summary:DashVector,
}

impl ShortLongMemory{
    async fn load_context(&self, max: usize) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
        let len = self.short_context.len();
        if len < max {
            self.short_context.clone().ok()
        } else {
            let mut list = Vec::with_capacity(max);
            for i in (len - max)..len {
                list.push(self.short_context[i].clone())
            }
            Ok(list)
        }
    }

    async fn add_session_log(&self,mut record: Vec<ChatCompletionRequestMessage>) {
        //fixme 存才并发错误
        #[allow(invalid_reference_casting)]
        unsafe {
            let sc = &mut *(&self.short_context as *const Vec<ChatCompletionRequestMessage> as *mut Vec<ChatCompletionRequestMessage>);
            sc.append(&mut record);
        }
    }

    async fn get_user_tag(&self,uid:&str, tag: &str) -> anyhow::Result<String> {
        let result = UIB.get(format!("{}-{}",uid,tag).as_str());
        return result
    }

    async fn set_user_tage(&self,uid:&str,kvs: HashMap<String, String>) {
        for (k,v) in kvs{
            UIB.set(format!("{}-{}",uid,k),v.as_str()).unwrap();
        }
    }
    async fn recall_summary(&self,uid:&str, query: &str, n: usize) -> anyhow::Result<Vec<String>> {
        self.summary.top_n(uid,query,n).await
    }
    async fn summary_history(&self,uid:&str){
        let prompt:ChatCompletionRequestMessage = ChatCompletionRequestSystemMessageArgs::default()
            .content("你是阅读理解专家。你的目标是，从过往的对话，提取关键信息，进行总结。总结后的内容必须言简意赅，简洁明了。最总通过`summery_tool`记录下来。")
            .build()
            .unwrap()
            .into();

        let mut context = vec![prompt];
        let mut chat_context = match self.load_context(1000).await{
            Ok(o) => o,
            Err(e) => {
                wd_log::log_field("error",e).error("summary history load context error");
                return;
            }
        };
        context.append(&mut chat_context);

        let summery_tool = ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(FunctionObjectArgs::default()
                .name("summery_tool")
                .description("记录总结内容")
                .parameters(json!({
                    "type": "object",
                    "properties": {
                        "list": {
                            "type": "array",
                            "items":{
                                "type":"string",
                                "description": "条目",
                            },
                            "description": "总结得到的列表",
                        },
                    },
                    "required": ["location"],
                }))
                .build().unwrap())
            .build().unwrap();


        let chat_req = CreateChatCompletionRequestArgs::default()
            .max_tokens(4096u16)
            .model("gpt-4")
            .messages(context)
            .tools([summery_tool])
            .build()
            .unwrap();

        let client = Client::new();
        let resp = client.chat().create(chat_req).await.unwrap();
        let mut summery_list = vec![];
        for i in resp.choices {
            if i.message.tool_calls.is_some(){
                for j in i.message.tool_calls.unwrap(){
                    println!("总结的内容：{}",j.function.arguments);
                    if let Ok(sl) = serde_json::from_str::<SummeryList>(j.function.arguments.as_str()) {
                        summery_list = sl.list;
                        break
                    }
                }
            }else{
                wd_log::log_field("role",i.message.role)
                    .field("message",format!("{:?}",i))
                    .info("summery not is tool");
            }
        }
        if let Err(e) = self.summary.insert(uid.to_string(),summery_list).await {
            wd_log::log_field("error",e).error("insert to dash vector error")
        }
    }
}

#[derive(Debug,Default)]
pub struct ShortLongMemoryMap{
    map:HashMap<String,ShortLongMemory>
}

#[async_trait::async_trait]
impl Memory for ShortLongMemoryMap{
    async fn load_context(&self, user: &str, max: usize) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
        if let Some(m) = self.map.get(user) {
            let result = m.load_context(max).await;
            return result
        }else{
            Ok(vec![])
        }
    }

    async fn add_session_log(&self, user: &str, record: Vec<ChatCompletionRequestMessage>) {
        if let Some(m) = self.map.get(user) {
            m.add_session_log(record).await;
        }else{
            //fixme 存才并发错误
            #[allow(invalid_reference_casting)]
            unsafe {
                let m = ShortLongMemory::default();
                m.add_session_log(record).await;
                let map = &mut *(&self.map as *const HashMap<String,ShortLongMemory> as * mut HashMap<String,ShortLongMemory>);
                map.insert(user.to_string(),m);
            }
        }
    }

    async fn get_user_tag(&self, user: &str, tag: &str) -> anyhow::Result<String> {
        if let Some(m) = self.map.get(user) {
            m.get_user_tag(user,tag).await
        }else{
            Ok(String::new())
        }
    }

    async fn set_user_tage(&self, user: &str, kvs: HashMap<String, String>) {
        if let Some(m) = self.map.get(user) {
            m.set_user_tage(user,kvs).await;
        }else{
            //fixme 存才并发错误
            #[allow(invalid_reference_casting)]
            unsafe {
                let m = ShortLongMemory::default();
                m.set_user_tage(user,kvs).await;
                let map = &mut *(&self.map as *const HashMap<String,ShortLongMemory> as * mut HashMap<String,ShortLongMemory>);
                map.insert(user.to_string(),m);
            }
        }
    }

    async fn recall_summary(&self, user: &str, query:  &str, n: usize) -> anyhow::Result<Vec<String>> {
        if let Some(m) = self.map.get(user) {
            m.recall_summary(user,query,n).await
        }else{
            Ok(vec![])
        }
    }

    async fn summary_history(&self, user: &str) {
        if let Some(m) = self.map.get(user) {
            m.summary_history(user).await;
        };
    }
}

impl ShortLongMemoryMap {
    pub fn init(&mut self,user:&str){
        if let None = self.map.get(user) {
            let memory = ShortLongMemory::default();
            self.map.insert(user.to_string(),memory);
        }
    }
    pub fn as_user_tag_tool(self:&Arc<Self>)->UserTagsNode{
        UserTagsNode{inner:self.clone()}
    }
    pub fn as_summery_tool(self:&Arc<Self>)->SummeryNode{
        SummeryNode{inner:self.clone()}
    }

}


pub struct UserTagsNode{
    inner:Arc<ShortLongMemoryMap>,
}
#[derive(Serialize,Deserialize)]
struct UserTagsNodeReq{
    tag:String,
    #[serde(default = "String::default")]
    name:String,
    #[serde(default = "usize::default")]
    age:usize,
}

impl UserTagsNode {
    pub fn as_openai_tool(&self) -> ChatCompletionTool {
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(FunctionObjectArgs::default()
                .name(self.id())
                .description("记录用户姓名，年龄")
                .parameters(Value::from_str(r#"{"type":"object","properties":{"tag":{"type":"string","description":"用户标签","enum":["name","age"]},"name":{"type":"string","description":"姓名"},"age":{"type":"integer","description":"年龄"}},"required":["tag"]}"#).unwrap())
                .build().unwrap())
            .build()
            .unwrap()
            .into()
    }
}

#[async_trait::async_trait]
impl Node for UserTagsNode {
    fn id(&self) -> String {
        "user_tag".into()
    }
    async fn go(&self, ctx: Arc<Context>, mut args: TaskInput) -> anyhow::Result<TaskOutput> {
        let uid = user_id_from_ctx(ctx.as_ref());
        if let Some(s) = args.get_value::<ChatCompletionMessageToolCall>() {
            let input = s.function.arguments;
            let req = serde_json::from_str::<UserTagsNodeReq>(input.as_str())?;
            match req.tag.as_str() {
                "name"=> self.inner.set_user_tage(uid.as_str(),HashMap::from([("name".into(),req.name)])).await,
                "age"=> self.inner.set_user_tage(uid.as_str(),HashMap::from([("age".into(),req.age.to_string())])).await,
                _=> return anyhow::anyhow!("tag[{}] unknown",req.tag).err()
            };
            let msg: ChatCompletionRequestMessage = ChatCompletionRequestToolMessageArgs::default()
                .tool_call_id(s.id)
                .content("success")
                .build()
                .unwrap()
                .into();
            return go_next_or_over(ctx,msg)
        }else{
            anyhow::anyhow!("args not find").err()
        }
    }
}

pub struct SummeryNode{
    inner:Arc<ShortLongMemoryMap>,
}
impl SummeryNode {
    pub fn as_openai_tool(&self) -> ChatCompletionTool {
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(FunctionObjectArgs::default()
                .name(self.id())
                .description("用户主动总结")
                .parameters(Value::from_str(r#"{"type":"object","properties":{"is":{"type":"boolean","description":"是否总结"}}}"#).unwrap())
                .build().unwrap())
            .build()
            .unwrap()
            .into()
    }
}

#[async_trait::async_trait]
impl Node for SummeryNode {
    fn id(&self) -> String {
        "summery".into()
    }

    async fn go(&self, ctx: Arc<Context>, mut args: TaskInput) -> anyhow::Result<TaskOutput> {
        let uid = user_id_from_ctx(ctx.as_ref());
        let input = args.get_value::<ChatCompletionMessageToolCall>().unwrap();
        self.inner.summary_history(uid.as_str()).await;
        let msg: ChatCompletionRequestMessage = ChatCompletionRequestToolMessageArgs::default()
            .tool_call_id(input.id)
            .content("success")
            .build()
            .unwrap()
            .into();
        go_next_or_over(ctx,msg)
    }
}