use crate::{CtxStatus, END_NODE_CODE, Flow, Output, Runtime};

impl Runtime{
    pub async fn middle_handle_ctx_over_callback(flow: Flow) -> anyhow::Result<Output>{
        let ctx = flow.ctx.clone();
        let res = flow.call().await;
        if let Some(ref lock) = ctx.over_callback {
            match ctx.meta.status() {
                CtxStatus::SUCCESS | CtxStatus::ERROR=>{
                    let mut lock = lock.lock().unwrap();
                    while let Some(i) = lock.pop(){
                        i(ctx.clone());
                    }
                }
                _=>{}
            }
        }
        return res
    }
    pub async fn middle_handle_waker_waiter(flow: Flow) -> anyhow::Result<Output>{
        if flow.code == END_NODE_CODE {
            let ws = flow.ctx.waker.clone();
            let code = flow.ctx.code.clone();
            let result = flow.call().await;
            if let Some(waker) = ws.remove(code.as_str()){
                waker.waker.wake();
            }
            result
        } else {
            flow.call().await
        }
    }
    pub async fn middle_handle_save_output_to_ctx(flow: Flow) -> anyhow::Result<Output>{
        let code = flow.code.clone();
        let ctx = flow.ctx.clone();
        let output = flow.call().await?;
        if output.raw_to_ctx {
            ctx.set_box(code,output.any);
        }else{
            ctx.set(code,output);
        }
        Ok(Output::null())
    }
    pub async fn middle_handle_status_check(flow: Flow) -> anyhow::Result<Output>{
        if flow.code == END_NODE_CODE {
            let ctx = flow.ctx.clone();
            let result = flow.call().await;
            if result.is_ok() {
                ctx.end_over::<()>(None);
            }
            result
        } else {
            flow.call().await
        }
    }
    pub(crate) fn register_default_middle_handles(self)->Self{
        self.register_middle_fn(Runtime::middle_handle_ctx_over_callback)
            .register_middle_fn(Runtime::middle_handle_waker_waiter)
            .register_middle_fn(Runtime::middle_handle_save_output_to_ctx)
            .register_middle_fn(Runtime::middle_handle_status_check)
    }
}