use crate::{CtxStatus, END_NODE_CODE, Flow, Output, RTError, Runtime};

impl Runtime{
    pub async fn middle_handle_ctx_over_callback(flow: Flow) -> anyhow::Result<Output>{
        let ctx = flow.ctx.clone();
        let res = flow.call().await;
        if let Some(ref lock) = ctx.over_callback {
            match ctx.status() {
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
        let ctx = flow.ctx.clone();
        let res = flow.call().await;
        let status = ctx.status();
        if status == CtxStatus::SUCCESS || status == CtxStatus::ERROR {
            if let Some(w) = ctx.runtime.waker.remove(ctx.code.as_str()){
                w.waker.wake();
            }
        }
        res
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
        //检查开始状态
        if flow.ctx.status() != CtxStatus::RUNNING {
            return RTError::ContextAbort.anyhow()
        }
        let ctx = flow.ctx.clone();
        let over = flow.code == END_NODE_CODE;
        let result = flow.call().await;
        match result {
            Ok(o) => {
                if over {
                    ctx.end_over::<()>(None);
                }
                Ok(o)
            }
            Err(e) => {
                ctx.error_over(RTError::UNKNOWN(format!("task error:{:?}",e)));
                Err(e)
            }
        }
    }
    pub async fn middle_handle_stack_check(flow: Flow) -> anyhow::Result<Output>{
        if flow.ctx.usable_stack() <= 0 {
            flow.ctx.error_over(RTError::UNKNOWN(format!("stack[{:?}] full", flow.ctx.stack)));
            return Ok(Output::null())
        }
        flow.call().await
    }
    pub(crate) fn register_default_middle_handles(self)->Self{
        self.register_middle_fn(Runtime::middle_handle_ctx_over_callback)
            .register_middle_fn(Runtime::middle_handle_waker_waiter)
            .register_middle_fn(Runtime::middle_handle_status_check)
            .register_middle_fn(Runtime::middle_handle_stack_check)
            .register_middle_fn(Runtime::middle_handle_save_output_to_ctx)
    }
}