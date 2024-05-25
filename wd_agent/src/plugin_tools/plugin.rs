use crate::plugin_tools::{ToolFunction, ToolHttp};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use wd_tools::PFErr;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Oauth {
    Header(HashMap<String, String>),
    Query(HashMap<String, String>),
    Oauth(),
    Tls(String, String), //私钥，公钥
}

#[derive(Clone)]
pub enum Tool {
    Http(ToolHttp),
    Python,
    Custom(Arc<dyn ToolFunction + Sync + 'static>),
}

impl Debug for Tool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Tool::Http(h) => h.fmt(f),
            Tool::Python => {
                write!(f, "[Tool.Python] this tool is python script")
            }
            Tool::Custom(_) => {
                write!(f, "[Tool.Custom] this tool is function")
            }
        }
    }
}
impl<T> From<T> for Plugin
where
    Tool: From<T>,
{
    fn from(value: T) -> Self {
        Plugin::default().add_tool("", value)
    }
}

pub enum PluginResult {
    Ok(Plugin),
    Err(anyhow::Error),
}

impl<T> From<T> for PluginResult
where
    T: TryInto<Tool, Error = anyhow::Error>,
{
    fn from(value: T) -> Self {
        match value.try_into() {
            Ok(o) => PluginResult::Ok(Plugin::default().add_tool("", o)),
            Err(e) => PluginResult::Err(e),
        }
    }
}

#[derive(Debug)]
pub struct ToolPython {
    pub import: Vec<String>,
    pub script: String,
}

#[derive(Debug, Clone, Default)]
pub struct Plugin {
    pub auth: Option<Oauth>,
    pub server: Option<(String, u16)>, //addr port
    pub tools: HashMap<String, Tool>,
}

impl Plugin {
    pub async fn call(mut self, tool_name: &str, args: String) -> anyhow::Result<String> {
        let tool = match self.tools.remove(tool_name) {
            Some(s) => s,
            None => return anyhow::anyhow!("plugin:api[{}] not found", tool_name).err(),
        };
        match tool {
            Tool::Http(htp) => {
                let (host, port) = if let Some(s) = self.server {
                    s
                } else {
                    // return anyhow::anyhow!("plugin:api[{}] http tool but grpc is nil",tool_name).err()
                    ("".to_string(), 0)
                };
                htp.call(host, port, args, self.auth).await
            }
            Tool::Python => {
                todo!()
            }
            Tool::Custom(function) => function.call(args).await,
        }
    }
    pub fn add_tool<S: Into<String>, T: Into<Tool>>(mut self, tool_name: S, tool: T) -> Self {
        self.tools.insert(tool_name.into(), tool.into());
        self
    }
    #[allow(dead_code)]
    pub fn set_auth<O: Into<Oauth>>(mut self, auth: O) -> Self {
        self.auth = Some(auth.into());
        self
    }
    #[allow(dead_code)]
    pub fn set_server<A: Into<String>>(mut self, host: A, port: u16) -> Self {
        self.server = Some((host.into(), port));
        self
    }
}
