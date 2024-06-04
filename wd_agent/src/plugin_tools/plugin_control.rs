use crate::plugin_tools::{Plugin, PluginResult};
use crate::rt_node_service::ToolEvent;
use std::collections::HashMap;
use wd_tools::{PFBox, PFErr, PFOk};

#[async_trait::async_trait]
pub trait PluginSchedule: Send {
    async fn schedule(&self, plugin_name: &str, tool_name: &str) -> anyhow::Result<Plugin>;
}

pub struct PluginControl {
    pub schedule: Box<dyn PluginSchedule + Sync + 'static>,
}
impl<T> From<T> for PluginControl
where
    T: PluginSchedule + Sync + 'static,
{
    fn from(value: T) -> Self {
        Self {
            schedule: Box::new(value),
        }
    }
}

#[async_trait::async_trait]
impl ToolEvent for PluginControl {
    async fn call(&self, name: &str, args: String) -> anyhow::Result<String> {
        let mut list = name.split('.').into_iter().rev().collect::<Vec<&str>>();
        let plugin_name = list.pop().unwrap_or("");
        let tool_name = list.pop().unwrap_or("");
        let plugin = self.schedule.schedule(plugin_name, tool_name).await?;
        plugin.call(tool_name, args).await
    }
}

#[derive(Default)]
pub struct PluginControlSchedule {
    map: HashMap<String, Plugin>,
    extend: Option<Box<dyn PluginSchedule + Sync + 'static>>,
}

impl PluginControlSchedule {
    pub fn try_register_plugin<S: Into<String>, T: Into<PluginResult>>(
        mut self,
        plugin_name: S,
        plugin: T,
    ) -> anyhow::Result<Self> {
        let plugin = match plugin.into() {
            PluginResult::Ok(o) => o,
            PluginResult::Err(e) => return Err(e),
        };
        self.map.insert(plugin_name.into(), plugin);
        self.ok()
    }
    pub fn register_plugin<S: Into<String>, T: Into<Plugin>>(
        mut self,
        plugin_name: S,
        plugin: T,
    ) -> Self {
        self.map.insert(plugin_name.into(), plugin.into());
        self
    }
    #[allow(dead_code)]
    pub fn set_extend_schedule<T: PluginSchedule + Sync + 'static>(mut self, tl: T) -> Self {
        self.extend = Some(tl.to_box());
        self
    }
    pub fn to_tool_event(self) -> impl ToolEvent {
        PluginControl::from(self)
    }
}

#[async_trait::async_trait]
impl PluginSchedule for PluginControlSchedule {
    async fn schedule(&self, plugin_name: &str, tool_name: &str) -> anyhow::Result<Plugin> {
        if let Some(s) = self.map.get(plugin_name) {
            return Ok(s.clone());
        }
        if let Some(ref schedule) = self.extend {
            return schedule.schedule(plugin_name, tool_name).await;
        } else {
            return anyhow::anyhow!("not found plugin_view[{}]", plugin_name).err();
        }
    }
}



#[cfg(test)]
mod test {
    use crate::plugin_tools::{init_py_rt_client, PluginControl, PluginControlSchedule};
    use crate::py;
    use crate::rt_node_service::ToolEvent;
    use crate::plugin_tools::ToolPython;

    const PY_SCRIPT_CODE:&'static str = r#"
def sms_send(msg):
    data=msg.data
    print("send msg:",data["content"])
    return {"id":data["id"],"result":"success"}
    "#;

    #[tokio::test]
    async fn test_schedule() {

        init_py_rt_client("http://127.0.0.1:50001").await;

        let plugin: PluginControl = PluginControlSchedule::default()
            .register_plugin("test_function_plugin", |x| async move {
                println!("test_function_plugin ===>{}", x);
                Ok(x)
            })
            .register_plugin("test_py_plugin",py!(PY_SCRIPT_CODE))
            .try_register_plugin("test_http_api", ("get", "https://www.baidu.com"))
            .unwrap()
            .into();

        let result = plugin
            .call("test_function_plugin.", "hello world".to_string())
            .await
            .unwrap();
        assert_eq!("hello world", result.as_str());

        let htp = plugin
            .call("test_http_api", "hello".to_string())
            .await
            .unwrap();
        println!("---->\n{htp}");

        let py = plugin
            .call("test_py_plugin.sms_send",r#"{"id":2024042701001,"content":"this is a py script"}"#.into())
            .await
            .unwrap();
        println!("---->\n{py}");
    }
}
