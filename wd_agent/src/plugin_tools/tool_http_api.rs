use crate::plugin_tools::{Oauth, Tool};
use std::any::Any;
use std::collections::HashMap;
use wd_tools::http::Method;
use wd_tools::{Base64URLEncode, PFErr, PFOk};

#[derive(Debug, Clone)]
pub struct ToolHttp {
    pub method: Method,
    pub path: String,
    pub query: String,
    pub header: HashMap<String, String>,
}

impl<Path, Query> TryFrom<(&str, Path, Query, &str)> for Tool
where
    Path: Into<String>,
    Query: Into<String>,
{
    type Error = anyhow::Error;

    fn try_from(
        (method, path, query, headers): (&str, Path, Query, &str),
    ) -> Result<Self, Self::Error> {
        let th = ToolHttp::new(method, path, query, headers)?;
        Tool::Http(th).ok()
    }
}
impl<Path> TryFrom<(&str, Path, &str)> for Tool
where
    Path: Into<String>,
{
    type Error = anyhow::Error;

    fn try_from((method, path, headers): (&str, Path, &str)) -> Result<Self, Self::Error> {
        let th = ToolHttp::new(method, path, "", headers)?;
        Tool::Http(th).ok()
    }
}

impl<Path> TryFrom<(&str, Path)> for Tool
where
    Path: Into<String>,
{
    type Error = anyhow::Error;

    fn try_from((method, path): (&str, Path)) -> Result<Self, Self::Error> {
        let th = ToolHttp::new(method, path, "", r#"{}"#)?;
        Tool::Http(th).ok()
    }
}

// impl<T> From<T> for Plugin
// where T:TryInto<Tool>
// {
//     fn from(value: T) -> Self {
//         let tool = value.try_into().unwrap();
//         Plugin::default()
//             .add_tool("",tool)
//     }
// }

impl ToolHttp {
    pub fn new<Path, Query>(
        method: &str,
        path: Path,
        query: Query,
        headers: &str,
    ) -> anyhow::Result<Self>
    where
        Path: Into<String>,
        Query: Into<String>,
    {
        let method = match Method::try_from(method.to_uppercase().as_str()) {
            Ok(o) => o,
            Err(e) => return anyhow::anyhow!("[ToolHttp.new] method error:{}", e).err(),
        };
        let this = Self {
            method,
            path: path.into(),
            query: query.into(),
            header: serde_json::from_str(headers)?,
        };
        Ok(this)
    }
    pub async fn call(
        self,
        host: String,
        port: u16,
        content: String,
        auth: Option<Oauth>,
    ) -> anyhow::Result<String> {
        let ToolHttp {
            method,
            path,
            query,
            header,
        } = self;

        let url = if port == 0 {
            if query.is_empty() {
                format!("{host}{path}")
            } else {
                format!("{host}{path}?{query}")
            }
        } else {
            if query.is_empty() {
                format!("{host}:{port}{path}")
            } else {
                format!("{host}:{port}{path}?{query}")
            }
        };

        let mut builder = wd_tools::http::Http::new(method.clone(), url).unwrap();

        for (k, v) in header {
            builder = builder.header(k, v);
        }

        if let Some(s) = auth {
            match s {
                Oauth::Header(map) => {
                    for (k, v) in map {
                        builder = builder.header(k, v);
                    }
                }
                Oauth::Query(qs) => {
                    let mut query = String::new();
                    for (k, v) in qs {
                        query.push_str(format!("{k}={v}").as_str());
                    }
                    builder.url.set_query(Some(query.as_str()));
                }
                //todo
                Oauth::Oauth() => {
                    return anyhow::anyhow!("tool http api is not support oauth").err()
                }
                //todo
                Oauth::Tls(_, _) => {
                    return anyhow::anyhow!("tool http api is not support tls auth").err()
                }
            }
        }

        if method == Method::GET {
            let content = content.base64_encode_url()?;
            builder
                .url
                .set_query(Some(format!("{content}={content}").as_str()))
        } else if method == Method::POST {
            builder = builder.body(content)
        } else {
            return anyhow::anyhow!("http api only support get and post method").err();
        };

        let resp = builder
            .hook_response(|_ctx, resp| async move {
                let s = resp.text().await.unwrap_or_else(|e| format!("{e}"));
                let bs: Box<dyn Any> = Box::new(s);
                Ok(bs)
            })
            .into_send::<String>()
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod test {
    use crate::plugin_tools::ToolHttp;

    #[test]
    fn test_new_http() {
        let ht =
            ToolHttp::new("get", "https://www.baidu.com/api/v1", "a=b", r#"{"c":"d"}"#).unwrap();
        println!("{:?}", ht)
    }
}
