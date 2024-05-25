use async_openai::types::CreateEmbeddingRequestArgs;
use bytes::Buf;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::connect::dns::GaiResolver;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;
use wd_tools::{EncodeHex, PFErr, PFOk, MD5};

#[derive(Debug, Serialize, Deserialize)]
struct DashVectorResp {
    code: isize,
    message: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct DashVectorCreateRequest {
    docs: Vec<DashVectorDoc>,
}

impl DashVectorCreateRequest {
    pub fn new(uid: String, docs: Vec<String>, vecs: Vec<Vec<f32>>) -> Self {
        let mut list = vec![];
        for i in docs.into_iter().zip(vecs).map(|x| (uid.clone(), x.0, x.1)) {
            list.push(i.into());
        }
        Self { docs: list }
    }
}
#[derive(Debug, Serialize, Deserialize)]
struct DashVectorDoc {
    id: String,
    vector: Vec<f32>,
    fields: Value,
}
impl From<(String, String, Vec<f32>)> for DashVectorDoc {
    fn from(value: (String, String, Vec<f32>)) -> Self {
        let (uid, content, vector) = value;
        let id = uid.as_bytes().md5().to_hex();
        let create_time = wd_tools::time::utc_timestamp() as i32;
        let fields = serde_json::to_value(json!({
            "uid" : uid,
            "create_time" : create_time,
            "content":content
        }))
        .unwrap();
        Self { id, vector, fields }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DashVectorRecallRequest {
    vector: Vec<f32>,
    topk: usize,
    filter: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DashVectorRecallResp {
    code: isize,
    message: String,
    #[serde(default = "Vec::new")]
    output: Vec<RecallItem>,
}
#[derive(Debug, Serialize, Deserialize)]
struct RecallItem {
    id: String,
    fields: RecallItemFields,
}
#[derive(Debug, Serialize, Deserialize)]
struct RecallItemFields {
    uid: String,
    create_time: isize,
    content: String,
}

#[derive(Debug)]
pub struct DashVector {
    cluster: String,
    api_key: String,
    client: Client<HttpsConnector<HttpConnector<GaiResolver>>, BoxBody<Bytes, Infallible>>,
}

impl Default for DashVector {
    fn default() -> Self {
        Self::new()
    }
}

impl DashVector {
    pub fn new() -> Self {
        let https = hyper_tls::HttpsConnector::new();
        let client = Client::builder(TokioExecutor::new()).build(https);
        let cluster = std::env::var("DASH_VECTOR_CLUSTER").unwrap();
        let api_key = std::env::var("DASH_VECTOR_API_KEY").unwrap();
        Self {
            cluster,
            api_key,
            client,
        }
    }

    pub async fn insert(&self, uid: String, text: Vec<String>) -> anyhow::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let querys = text.iter().map(|x| x.as_str()).collect::<Vec<&str>>();
        let vecs = Self::embedding_small_1536(querys).await?;
        let req = DashVectorCreateRequest::new(uid, text, vecs);
        self.insert_api(req).await
    }
    pub async fn top_n(&self, uid: &str, query: &str, topk: usize) -> anyhow::Result<Vec<String>> {
        let mut vector = Self::embedding_small_1536(vec![query]).await?;
        let filter = format!("uid = '{}'", uid);
        let req = DashVectorRecallRequest {
            vector: vector.pop().unwrap(),
            topk,
            filter,
        };
        self.recall(req).await
    }

    async fn recall(&self, req: DashVectorRecallRequest) -> anyhow::Result<Vec<String>> {
        let req = Request::post(format!(
            "https://{}/v1/collections/summery_coll/query",
            self.cluster
        ))
        .header("Content-Type", "application/json")
        .header("dashvector-auth-token", self.api_key.clone())
        .body(Self::full(serde_json::to_string(&req)?))
        .unwrap();
        let resp = self.client.request(req).await?;
        let body = resp.collect().await?.aggregate();
        let resp: DashVectorRecallResp = serde_json::from_reader(body.reader())?;
        if resp.code != 0 {
            return anyhow::anyhow!("insert vector failed:{:?}", resp).err();
        }
        let mut list = vec![];
        for i in resp.output {
            list.push(i.fields.content);
        }
        return Ok(list);
    }

    fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Infallible> {
        Full::new(chunk.into()).boxed()
    }

    async fn insert_api(&self, req: DashVectorCreateRequest) -> anyhow::Result<()> {
        let req = Request::post(format!(
            "https://{}/v1/collections/summery_coll/docs",
            self.cluster
        ))
        .header("Content-Type", "application/json")
        .header("dashvector-auth-token", self.api_key.clone())
        .body(Self::full(serde_json::to_string(&req)?))
        .unwrap();
        let resp = self.client.request(req).await?;
        let body = resp.collect().await?.aggregate();
        let resp: DashVectorResp = serde_json::from_reader(body.reader())?;
        if resp.code == 0 {
            return Ok(());
        } else {
            return anyhow::anyhow!("insert vector failed:{:?}", resp).err();
        }
    }

    pub async fn embedding_small_1536(query: Vec<&str>) -> anyhow::Result<Vec<Vec<f32>>> {
        let len = query.len();
        let req = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-small") // text-embedding-3-small:1536
            .input(query)
            .build()?;
        let client = async_openai::Client::new();
        let resp = client.embeddings().create(req).await?;
        let mut list = vec![vec![]; len];
        for i in resp.data {
            list[i.index as usize] = i.embedding
        }
        return list.ok();
    }
}

#[cfg(test)]
mod test {
    use crate::pkg::dash_vector::DashVector;
    use http_body_util::Empty;
    use hyper::body::Bytes;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;

    #[tokio::test]
    async fn test_hyper_https() {
        let https = hyper_tls::HttpsConnector::new();
        let client = Client::builder(TokioExecutor::new()).build::<_, Empty<Bytes>>(https);
        let resp = client
            .get("https://www.baidu.com".parse().unwrap())
            .await
            .unwrap();
        println!("resp-->{:?}", resp);
    }

    #[tokio::test]
    async fn test_insert_api() {
        let dv = DashVector::new();
        dv.insert("001".to_string(), vec!["我叫王大锤".to_string()])
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn test_recall() {
        let dv = DashVector::new();
        let item = dv
            .top_n("001".into(), "我叫王大锤".into(), 1)
            .await
            .unwrap();
        println!("{:?}", item)
    }
}
