use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use wd_tools::http::Method;
use wd_tools::PFErr;
use wd_agent::plugin_tools::{Plugin, Tool, ToolFunction};

pub struct GooglePlugin{
    app_key : String
}

impl Into<Plugin> for GooglePlugin{
    fn into(self) -> Plugin {
        let gp = Arc::new(self);
        let mut tools = HashMap::new();
        tools.insert("search".to_string(),Tool::Custom(Arc::new(gp.search())));

        Plugin{
            tools,
            auth: None,
            server: None,
        }
    }
}

impl Default for GooglePlugin{
    fn default() -> Self {
        let app_key = std::env::var("GOOGLE_APP_KEY").unwrap_or("".into());
        GooglePlugin{app_key}
    }
}

impl GooglePlugin {
    #[allow(unused)]
    pub fn new<A:Into<String>>(app_key:A)->Self{
        let app_key = app_key.into();
        Self{app_key}
    }
    pub fn search(self:Arc<Self>)->impl ToolFunction + Sync + 'static{
        struct GooglePluginSearch(Arc<GooglePlugin>);
        #[derive(Debug, Default, Clone, Deserialize, Serialize)]
        struct GooglePluginSearchRequest{
            q:String,
            hl:Option<String>,
            engine:Option<String>,
            num:Option<usize>
        }
        #[async_trait::async_trait]
        impl ToolFunction for GooglePluginSearch {
            async fn call(&self, args: String) -> anyhow::Result<String> {
                let input = match serde_json::from_slice::<GooglePluginSearchRequest>(args.as_bytes()) {
                    Ok(o)=>o,
                    Err(e)=>{
                        return anyhow::anyhow!("google search error:{}",e).err();
                    }
                };
                if input.q.is_empty() {
                    return anyhow::anyhow!("google search q is nil").err();
                }
                let api_key = self.0.clone();
                let result:String = wd_tools::http::Http::new(Method::GET, "https://serpapi.com/search")?
                    .hook_client_build(|_,cb|{
                        let client = cb.timeout(Duration::from_secs(30)).build()?;
                        Ok(client)
                    })
                    .hook_request_build(move |_, mut req| {
                        req = req.query(&[("q", input.q.as_str()),("api_key",api_key.app_key.as_str()),("output","json")]);
                        if let Some(ref s) = input.engine {
                            req = req.query(&[("engine", s)]);
                        } else {
                            req = req.query(&[("engine", "google")]);
                        }

                        if let Some(ref s) = input.hl {
                            req = req.query(&[("hl", s)]);
                        } else {
                            req = req.query(&[("hl", "zh-cn")]);
                        }
                        if let Some(ref i) = input.num{
                            req = req.query(&[("num",i.to_string().as_str())]);
                        }

                        Ok(req)
                    })
                    .hook_response(|_, resp| async move {
                        let body = resp.text().await?;
                        Ok(Box::new(body) as Box<dyn Any>)
                    })
                    .into_send().await?;
                Ok(result)
            }
        }
        GooglePluginSearch(self)
    }
}