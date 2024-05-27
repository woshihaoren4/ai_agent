mod grpc;
#[cfg(feature = "grpc")]
pub mod proto;
mod py_runtime;

#[cfg(feature = "grpc")]
pub use grpc::common;

#[cfg(feature = "server")]
pub use grpc::server;
#[cfg(feature = "client")]
pub use grpc::client;

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use crate::grpc;

    //cargo test tests::build
    #[test]
    fn build() {
        println!("build success");
    }

    /// ## example 1
    /// {
    //   "src": 1,
    //   "module_name": "sys_info",
    //   "sys_path":"./custom_plugin",
    //   "function_name": "get_system_info",
    //   "function_input": {
    //     "fields": {
    //       "hello": {
    //         "kind": "world"
    //       }
    //     }
    //   }
    // }

    /// ## example 2
    /// {
    //   "src": 1,
    //   "module_name": "sys_info",
    //   "sys_path": "custom_plugin",
    //   "function_name": "generate_system_report",
    //   "function_input": {
    //     "fields": {
    //       "cpu_count": {
    //         "numberValue": 12,
    //         "kind": "numberValue"
    //       },
    //       "cpu_percent": {
    //         "numberValue": 23.5,
    //         "kind": "numberValue"
    //       },
    //       "disk_free": {
    //         "numberValue": 159.18515014648438,
    //         "kind": "numberValue"
    //       },
    //       "disk_total": {
    //         "numberValue": 465.62699127197266,
    //         "kind": "numberValue"
    //       },
    //       "disk_used": {
    //         "numberValue": 23.568099975585938,
    //         "kind": "numberValue"
    //       },
    //       "memory_available": {
    //         "numberValue": 13596.53515625,
    //         "kind": "numberValue"
    //       },
    //       "memory_total": {
    //         "numberValue": 32768,
    //         "kind": "numberValue"
    //       },
    //       "os_release": {
    //         "stringValue": "21.3.0",
    //         "kind": "stringValue"
    //       },
    //       "os_type": {
    //         "stringValue": "Darwin",
    //         "kind": "stringValue"
    //       }
    //     }
    //   }
    // }
    //
    /// ## example 3
    /// {
    //   "src": 0,
    //   "SCRIPT_CODE": "import sys\n\ndef handle(input):\n  print(\"python =>\",input)\n  version = sys.version\n  return {\"code\":0,\"msg\":\"success\",\"version\":version}\n",
    //   "module_name": "handle",
    //   "function_name": "handle",
    //   "function_input": {
    //     "fields": {
    //       "hello": {
    //         "kind": "world"
    //       }
    //     }
    //   }
    // }
    //
    /// bash: cargo test tests::run_server -- --nocapture
    #[tokio::test]
    async fn run_server() {
        grpc::server::Server::default()
            .run("[::1]:50001")
            .await
            .unwrap();
    }

    #[derive(Debug,Default,Serialize)]
    struct PyRequest{
        input:String
    }

    #[derive(Debug,Default,Deserialize)]
    struct PyResponse{
        version:String,
    }

    const SCRIPT_CODE:&'static str = r#"
import sys

def handle(input):
    print("python =>",input)
    version = sys.version
    return {"version":version}
    "#;

    #[tokio::test]
    async fn run_client() {
        let resp = grpc::client::Client::new("http://127.0.0.1:50001")
            .await.unwrap()
            .eval_script_code::<_,_,_,PyResponse>(SCRIPT_CODE,"handle",serde_json::json!({
                "input":"hello world"
            }))
            .await
            .unwrap();
        assert_eq!(true,resp.version.contains("3.11.9"));
        println!("--->{:?}",resp);
    }
}
