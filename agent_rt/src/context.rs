use crate::{Error, Node, Output, Plan, Runtime};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::DerefMut;
use std::sync::Arc;
use std::task::Waker;
use wd_tools::sync::Am;
use wd_tools::PFErr;

pub struct Context {
    pub ce: Arc<Am<ContextEntity>>,
}
impl Clone for Context {
    fn clone(&self) -> Self {
        let ce = self.ce.clone();
        Self { ce }
    }
}
fn type_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}
fn type_of_none<T>() -> &'static str {
    std::any::type_name::<T>()
}
pub struct ContextEntity {
    pub status: CtxStatus,
    pub waker: Option<Waker>,
    pub rt: Runtime,
    pub plan: Box<dyn Plan + Sync + 'static>,
    pub vars: HashMap<String, Box<dyn Any>>,
}
impl ContextEntity {
    pub fn new<P: Plan + Sync + 'static>(rt: Runtime, p: P) -> Self {
        let status = CtxStatus::default();
        let waker = None;
        ContextEntity {
            status,
            rt,
            waker,
            plan: Box::new(p),
            vars: HashMap::new(),
        }
    }
    pub fn build(self) -> Context {
        Context {
            ce: Arc::new(Am::new(self)),
        }
    }
}

impl Context {
    pub fn lock<Out>(&self, ctx_handle: impl FnOnce(&mut ContextEntity) -> Out) -> Out {
        let mut lock = self.ce.synchronize();
        ctx_handle(lock.deref_mut())
    }
    pub async fn ctx<Out>(&self, ctx_handle: impl FnOnce(&mut ContextEntity) -> Out) -> Out {
        let mut lock = self.ce.lock().await;
        ctx_handle(lock.deref_mut())
    }
    pub fn is_running(&self) -> bool {
        self.lock(|c| {
            if let CtxStatus::RUNNING(ref _w) = c.status {
                true
            } else {
                false
            }
        })
    }
    pub fn set_box_any<S: Into<String>>(self, key: S, b: Box<dyn Any>) -> Self {
        self.lock(|c| {
            c.vars.insert(key.into(), b);
        });
        self
    }
    pub fn set<S: Into<String>, T: Any>(self, key: S, t: T) -> Self {
        self.set_box_any(key.into(), Box::new(t))
    }
    pub async fn remove<T: Any>(&self, key: &str) -> anyhow::Result<T> {
        self.ctx(|c| {
            let val = if let Some(s) = c.vars.get(key) {
                s
            } else {
                return anyhow::anyhow!("Context.vars[{key}] not found").err();
            };
            let opt = val.downcast_ref::<T>();
            if opt.is_none() {
                return anyhow::anyhow!("Context.vars[{key}] type assert failed").err();
            }
            let val = c.vars.remove(key).unwrap();
            let box_val: Box<T> = val.downcast().unwrap();
            Ok(*box_val)
        })
        .await
    }
    pub(crate) fn error<E: Into<anyhow::Error>>(&self, err: E) {
        let status = CtxStatus::Error(err.into());
        self.set_status(status);
    }
    pub(crate) fn success(&self) {
        let status = CtxStatus::SUCCESS;
        self.set_status(status);
    }
    pub(crate) fn set_status(&self, mut status: CtxStatus) {
        self.lock(|c| {
            unsafe { std::ptr::swap(&mut c.status, &mut status) };
            if let CtxStatus::RUNNING(waker) = status {
                waker.wake();
            }
        });
    }
    pub fn into_status(&self) -> CtxStatus {
        let mut status = CtxStatus::Over;
        self.lock(|c| {
            unsafe { std::ptr::swap(&mut c.status, &mut status) };
        });
        status
    }
    pub async fn next(self, mut node: Node) -> anyhow::Result<Output> {
        let rt = self.ctx(|c| c.rt.clone()).await;
        loop {
            if node.middle_index < rt.entity.service_middles.len() {
                let middle = &rt.entity.service_middles[node.middle_index];
                node.middle_index += 1;
                if !middle.filter(&node) {
                    continue;
                }
                return middle.call(self, node).await;
            } else if node.middle_index == rt.entity.service_middles.len() {
                let service = node.service.clone();
                return service.call(self, node).await;
            } else {
                //Will not execute
                wd_log::log_error_ln!("[agent_rt] Context.next to will not execute");
                return Error::NextNodeNull.into();
            }
        }
    }
    // pub async fn launch<I:Any>(self,input: I)->anyhow::Result<()>{
    //     Runtime::launch(self,input).await
    // }
    pub async fn go<In: Any, Out: Any>(self, input: In) -> anyhow::Result<Out> {
        Runtime::go(self, input).await
    }
}
// impl Deref for Context {
//     type Target = Arc<ContextEntity>;
//
//     fn deref(&self) -> &Self::Target {
//         &self.ce
//     }
// }

#[derive(Default)]
pub enum CtxStatus {
    #[default]
    New,
    RUNNING(Waker),
    SUCCESS,
    Error(anyhow::Error),
    Over,
}
impl Display for CtxStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CtxStatus::New => write!(f, "CtxStatus::New"),
            CtxStatus::RUNNING(_) => write!(f, "CtxStatus::RUNNING"),
            CtxStatus::SUCCESS => write!(f, "CtxStatus::SUCCESS"),
            CtxStatus::Error(_) => write!(f, "CtxStatus::Error"),
            CtxStatus::Over => write!(f, "CtxStatus::Over"),
        }
    }
}
impl PartialEq for CtxStatus {
    fn eq(&self, other: &Self) -> bool {
        match other {
            CtxStatus::New => match self {
                CtxStatus::New => true,
                _ => true,
            },
            CtxStatus::RUNNING(_) => match self {
                CtxStatus::RUNNING(_) => true,
                _ => false,
            },
            CtxStatus::SUCCESS => match self {
                CtxStatus::SUCCESS => true,
                _ => false,
            },
            CtxStatus::Error(_) => match self {
                CtxStatus::Error(_) => true,
                _ => false,
            },
            CtxStatus::Over => match self {
                CtxStatus::Over => true,
                _ => false,
            },
        }
    }
}
