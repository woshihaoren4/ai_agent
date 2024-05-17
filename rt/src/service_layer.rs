use crate::{Context, Flow, Output, Runtime, Service};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use wd_tools::{PFErr, PFOk};

pub trait TestLayer<A, B>: Send + Sync {
    fn call(&self, a: A, b: B);
}

#[async_trait::async_trait]
pub trait ServiceLayer: Sync + Send {
    type Config;
    type Output;
    async fn call(
        &self,
        code: String,
        ctx: Arc<Context>,
        cfg: Self::Config,
    ) -> anyhow::Result<Self::Output>;
}

pub struct LayerJson<T, In, Out> {
    handle: T,
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

#[async_trait::async_trait]
impl<T, In, Out> Service for LayerJson<T, In, Out>
where
    T: ServiceLayer<Config = In, Output = Out>,
    In: for<'a> Deserialize<'a> + Send + Sync,
    Out: Serialize + Send + Sync,
{
    async fn call(&self, flow: Flow) -> anyhow::Result<Output> {
        let Flow {
            ctx,
            code,
            node_config,
            node_type_id,
            ..
        } = flow;
        let node_info = format!(
            "task[{}],node_type_id[{}],code[{}]",
            ctx.code, node_type_id, code
        );
        let cfg = match serde_json::from_str::<In>(node_config.as_str()) {
            Ok(o) => o,
            Err(e) => {
                return anyhow::anyhow!(
                    "code[{}],type parse failed, data[{}] error:{}",
                    node_info,
                    node_config.as_str(),
                    e
                )
                .err()
            }
        };
        let output = self.handle.call(code, ctx, cfg).await?;
        let raw = match serde_json::to_value(&output) {
            Ok(o) => o,
            Err(e) => return anyhow::anyhow!("code[{}],output json error:{}", node_info, e).err(),
        };
        Output::new(raw).raw_to_ctx().ok()
    }
}

impl<T, In, Out, Fut> From<T> for LayerJson<LayerJsonFn<T, In, Out, Fut>, In, Out> {
    fn from(value: T) -> Self {
        let json_fn = LayerJsonFn::new(value);
        Self::new(json_fn)
    }
}

impl<T, In, Out> LayerJson<T, In, Out> {
    pub fn new(handle: T) -> Self {
        LayerJson {
            handle,
            _in: PhantomData::default(),
            _out: PhantomData::default(),
        }
    }
    // pub fn from_fn<Fut>(function:T)->LayerJson<LayerJsonFn<T, In, Out, Fut>, In, Out>{
    //     let inner = LayerJson::new(function);
    //     let json_fn = LayerJsonFn::<T, In, Out, Fut> { inner, _fut: PhantomData::default() };
    //     LayerJson::new(json_fn)
    // }
}

pub struct LayerJsonFn<T, In, Out, Fut> {
    inner: LayerJson<T, In, Out>,
    _fut: PhantomData<Fut>,
}

#[async_trait::async_trait]
impl<T, In, Out, Fut> ServiceLayer for LayerJsonFn<T, In, Out, Fut>
where
    T: Fn(String, Arc<Context>, In) -> Fut + Send + Sync,
    Fut: Future<Output = anyhow::Result<Out>> + Send + Sync,
    In: for<'a> Deserialize<'a> + Send + Sync,
    Out: Serialize + Send + Sync,
{
    type Config = In;
    type Output = Out;

    async fn call(
        &self,
        code: String,
        ctx: Arc<Context>,
        cfg: Self::Config,
    ) -> anyhow::Result<Self::Output> {
        (self.inner.handle)(code, ctx, cfg).await
    }
}

impl<T, In, Out, Fut> LayerJsonFn<T, In, Out, Fut> {
    pub fn new(function: T) -> Self {
        let inner = LayerJson::new(function);
        Self {
            inner,
            _fut: PhantomData::default(),
        }
    }
}

impl Runtime {
    pub fn register_service_layer<ID: Into<String>, T, I, O, F: Into<LayerJson<T, I, O>>>(
        self,
        id: ID,
        layer_fn: F,
    ) -> Self
    where
        T: ServiceLayer<Config = I, Output = O> + 'static,
        I: for<'a> serde::Deserialize<'a> + Send + Sync + 'static,
        O: Serialize + Send + Sync + 'static,
    {
        self.register_service(id.into(), layer_fn.into())
    }
}

#[cfg(test)]
mod test {
    use crate::{Context, LayerJson, Output, PlanBuilder, Runtime, ServiceLayer, END_NODE_CODE};
    use serde_json::Value;
    use std::sync::Arc;
    use wd_tools::{PFArc, PFOk};

    pub struct LayerTest {}

    #[async_trait::async_trait]
    impl ServiceLayer for LayerTest {
        type Config = Value;
        type Output = Value;

        async fn call(
            &self,
            _: String,
            ctx: Arc<Context>,
            cfg: Self::Config,
        ) -> anyhow::Result<Self::Output> {
            println!("config ---> {}", cfg);
            Value::String("success".into()).ok()
        }
    }

    async fn service_layer_fn_show(
        _: String,
        ctx: Arc<Context>,
        cfg: Value,
    ) -> anyhow::Result<Value> {
        println!("async fn config ---> {}", cfg);
        Value::String("fn success".into()).ok()
    }

    //cargo test service_layer::test::test_layer_json -- --nocapture
    #[tokio::test]
    pub async fn test_layer_json() {
        let rt = Runtime::default()
            .register_service("layer_test", LayerJson::new(LayerTest {}))
            .register_service_layer("layer_fn", service_layer_fn_show)
            .register_service_layer(
                "lambda_fn",
                |code: String, ctx: Arc<Context>, _: Value| async move {
                    println!("lambda_fn code ---> {}", code);
                    Value::String("lambda success".into()).ok()
                },
            )
            .launch();

        let plan = PlanBuilder::start(
            (END_NODE_CODE, "layer_test", r#"{"key":"hello"}"#),
            vec![""],
        )
        .check_and_build()
        .unwrap();
        let res = rt
            .ctx("test001", plan)
            .arc()
            .block_on::<Value>()
            .await
            .unwrap();
        println!("{:?}", res.to_string());
        assert_eq!(r#""success""#, res.to_string());

        let plan = PlanBuilder::start((END_NODE_CODE, "layer_fn", r#"{"key":"world"}"#), vec![""])
            .check_and_build()
            .unwrap();
        let res = rt
            .ctx("test001", plan)
            .arc()
            .block_on::<Value>()
            .await
            .unwrap();
        println!("{:?}", res.to_string());
        assert_eq!(r#""fn success""#, res.to_string());

        let plan = PlanBuilder::start((END_NODE_CODE, "lambda_fn", r#"{}"#), vec![""])
            .check_and_build()
            .unwrap();
        let res = rt
            .ctx("test001", plan)
            .arc()
            .block_on::<Value>()
            .await
            .unwrap();
        println!("{:?}", res.to_string());
        assert_eq!(r#""lambda success""#, res.to_string())
    }
}
