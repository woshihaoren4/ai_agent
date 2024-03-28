use crate::consts::{go_next_or_over, AGENT_TOOL_WRAPPER};
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestMessage,
    ChatCompletionRequestToolMessageArgs, ChatCompletionTool, ChatCompletionToolArgs,
    ChatCompletionToolType, FunctionObject, FunctionObjectArgs,
};
use rt::{Context, Node, TaskInput, TaskOutput};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use wd_tools::{PFErr, PFOk};

pub struct ToolNode {
    id: String,
    meta: FunctionObject,
    // handle:Box<dyn Fn(String)->Box<Pin<dyn Future<Output=anyhow::Result<String>>>>>
    handle: Box<
        dyn Fn(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>>
            + Send
            + Sync
            + 'static,
    >,
}
impl ToolNode {
    pub fn new<S: Into<String>, D: Into<String>, F>(id: S, desc: D, param: &str, handle: F) -> Self
    where
        F: Fn(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let id = id.into();
        let handle = Box::new(handle);
        let meta = FunctionObjectArgs::default()
            .name(id.clone())
            .description(desc.into())
            .parameters(Value::from_str(param).unwrap())
            .build()
            .unwrap();
        Self { id, meta, handle }
    }
    pub fn as_openai_tool(&self) -> ChatCompletionTool {
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(self.meta.clone())
            .build()
            .unwrap()
            .into()
    }
}

impl ToolNode {
    #[allow(dead_code)]
    pub(crate) fn mock_get_current_weather() -> Self {
        ToolNode::new(
            "get_current_weather",
            "Get the current weather in a given location",
            r#"{"type":"object","properties":{"location":{"type":"string","description":"the city","enum":["beijing","shanghai"]}},"required":["location"]}"#,
            Self::get_current_weather,
        )
    }
    #[allow(dead_code)]
    pub(crate) fn mock_taobao() -> Self {
        ToolNode::new(
            "submit_order",
            "在线购物",
            r#"{"type":"object","properties":{"product":{"type":"string","description":"商品名称"}},"required":["product"]}"#,
            Self::submit_order,
        )
    }
    #[allow(dead_code)]
    fn get_current_weather(
        input: String,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>> {
        Box::pin(async move {
            let val = Value::from_str(input.as_str())?;
            let val = if let Value::Object(mut map) = val {
                map.remove("location")
            } else {
                return anyhow::anyhow!("get_current_weather args parems failed").err();
            };
            let location = if let Some(location) = val {
                location
            } else {
                return anyhow::anyhow!("not find location arg").err();
            };
            let location = if let Value::String(s) = location {
                s
            } else {
                return anyhow::anyhow!("location arg type failed").err();
            };
            match location.as_str() {
                "beijing"=>Ok(r#"{"data":[{"wind_dir_night":"北风","wind_level_night":"1","condition":"多云","temp_low":4,"weather_day":"多云","humidity":62,"temp_high":15,"wind_dir_day":"北风","wind_level_day":"3","predict_date":"2024-03-12","predictDate":"2024-03-12"}]}"#.to_string()),
                "shanghai"=>Ok(r#"{"data":[{"wind_dir_night":"南风","wind_level_night":"2","condition":"大雨","temp_low":4,"weather_day":"晴","humidity":55,"temp_high":16,"wind_dir_day":"南风","wind_level_day":"2","predict_date":"2024-03-12","predictDate":"2024-03-12"}]}"#.to_string()),
                _ =>{
                    anyhow::anyhow!("location unknown").err()
                }
            }
        })
    }
    #[allow(dead_code)]
    fn submit_order(input: String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>> {
        Box::pin(async move {
            let val = Value::from_str(input.as_str())?;
            let val = if let Value::Object(mut map) = val {
                map.remove("product")
            } else {
                return anyhow::anyhow!("submit_order args parems failed").err();
            };
            let product = if let Some(product) = val {
                product
            } else {
                return anyhow::anyhow!("not find product arg").err();
            };
            let product = if let Value::String(s) = product {
                s
            } else {
                return anyhow::anyhow!("product arg type failed").err();
            };
            match product.as_str() {
                "雨伞" => Ok(r#"{"code":0,"msg":"success","id":749342034857820}"#.to_string()),
                _ => anyhow::anyhow!("product unknown").err(),
            }
        })
    }
}

#[async_trait::async_trait]
impl Node for ToolNode {
    fn id(&self) -> String {
        self.id.clone()
    }

    async fn go(&self, ctx: Arc<Context>, mut args: TaskInput) -> anyhow::Result<TaskOutput> {
        if let Some(input) = args.get_value::<String>() {
            let resp = (self.handle)(input).await?;
            return go_next_or_over(ctx, resp);
        }
        if let Some(call) = args.get_value::<ChatCompletionMessageToolCall>() {
            let input = call.function.arguments;
            let resp = (self.handle)(input).await?;

            let msg: ChatCompletionRequestMessage = ChatCompletionRequestToolMessageArgs::default()
                .tool_call_id(call.id)
                .content(resp)
                .build()
                .unwrap()
                .into();

            return go_next_or_over(ctx, msg);
        };
        return anyhow::anyhow!("tool args error").err();
    }
}

#[derive(Debug, Default, Clone)]
pub struct AgentTool {
    agent_id: String,
    description: String,
}
impl AgentTool {
    pub fn new(agent_id: String, description: String) -> Self {
        Self {
            agent_id,
            description,
        }
    }
    pub fn get_agent_id(&self) -> &str {
        self.agent_id.as_str()
    }
    pub fn parameters() -> Value {
        Value::from_str(r#"{"type":"object","properties":{"input":{"type":"string","description":"user input"}}}"#).unwrap()
    }
    pub fn as_openai_tool(&self) -> ChatCompletionTool {
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name(self.id())
                    .parameters(Self::parameters())
                    .description(self.description.clone())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
            .into()
    }
}
#[async_trait::async_trait]
impl Node for AgentTool {
    fn id(&self) -> String {
        format!("{}_{}", AGENT_TOOL_WRAPPER, self.agent_id)
    }

    async fn go(&self, _ctx: Arc<Context>, _args: TaskInput) -> anyhow::Result<TaskOutput> {
        TaskOutput::new(self.agent_id.clone(), 1usize).ok()
    }
}

#[cfg(test)]
mod test {
    use crate::tool::ToolNode;
    use async_openai::types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestMessage, FunctionCall,
    };
    use rt::{Context, Node, TaskInput};
    use wd_tools::PFArc;

    #[tokio::test]
    async fn test_tool() {
        let tool = ToolNode::mock_get_current_weather();

        let mut x = tool
            .go(
                Context::new("get_current_weather").arc(),
                TaskInput::from_value(r#"{"location":"shanghai"}"#.to_string()),
            )
            .await
            .unwrap();
        let info = x.get_value::<String>().unwrap();
        println!("result --->{}", info)
    }

    #[tokio::test]
    async fn test_chat_msg() {
        let tool = ToolNode::mock_get_current_weather();

        let req = ChatCompletionMessageToolCall {
            id: "1".to_string(),
            r#type: Default::default(),
            function: FunctionCall {
                name: "get_current_weather".to_string(),
                arguments: r#"{"location":"shanghai"}"#.to_string(),
            },
        };

        let mut x = tool
            .go(
                Context::new("get_current_weather").arc(),
                TaskInput::from_value(req),
            )
            .await
            .unwrap();
        let info = x.get_value::<ChatCompletionRequestMessage>().unwrap();
        println!("{:?}", info)
    }
}
