use std::str::FromStr;
use wd_tools::PFErr;
use python_rt::client::Client;
use python_rt::proto::proto::{CallFunctionRequest};
use crate::plugin_tools::{Oauth, Tool};


pub static mut CLIENT:Option<Client> = None;
#[allow(dead_code)]
pub async fn init_py_rt_client(url:&str){
    unsafe {
        let client = Client::new(url).await.expect("init_py_rt_client failed");
        CLIENT = Some(client)
    }
}
#[allow(dead_code)]
pub fn get_py_rt_client()->Client{
    unsafe {
        if let Some(ref s) = CLIENT{
            s.clone()
        }else {
            panic!("CLIENT not init")
        }
    }
}

#[derive(Clone)]
pub struct ToolPython{
    pub req:CallFunctionRequest
}

impl From<ToolPython> for Tool{
    fn from(value: ToolPython) -> Self {
        Tool::Python(value)
    }
}

#[macro_export]
macro_rules! py {
    ($code:tt) => {
        ToolPython::new_script_code($code)
    };
    ($sys_path:tt,$module:tt)=>{
        ToolPython::from_module($sys_path,$module)
    };
}


impl ToolPython{
    pub fn new_script_code<S:Into<String>>(code:S)->Self{
        let req = CallFunctionRequest{
            src: 0,
            script_code: Some(code.into()),
            module_name: "default_module".to_string(),
            file_name: None,
            sys_path: None,
            function_name: "".into(),
            function_input: None,
        };

        Self{req }
    }
    #[allow(dead_code)]
    pub fn from_module<S:Into<String>,M:Into<String>>(sys_path:S,module_name:M)->Self{
        let req = CallFunctionRequest{
            src: 1,
            script_code: None,
            module_name: module_name.into(),
            file_name: None,
            sys_path: Some(sys_path.into()),
            function_name: "".into(),
            function_input: None,
        };

        Self{req }
    }

    pub async fn call(
        mut self,
        function_name:&str, mut args:String,
        _auth: Option<Oauth>,
    ) -> anyhow::Result<String> {
        if args.is_empty() {
            args = "{}".into()
        }
        let value = serde_json::Value::from_str(args.as_str())?;
        self.req.function_input = python_rt::common::serde_value_to_prost_struct(&value);
        self.req.function_name = function_name.to_string();

        let client = get_py_rt_client();
        let resp = client.call_function(self.req).await?;
        if resp.code != 0 {
            return anyhow::anyhow!("exec python error:{}",resp.msg).err()
        }
        if let Some(s) = resp.output{
            let value = python_rt::common::prost_struct_to_serde_value(s);
            let out = serde_json::to_string(&value)?;
            Ok(out)
        }else{
            Ok(resp.msg)
        }
    }
}