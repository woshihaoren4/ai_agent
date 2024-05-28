use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::Python;
use serde_json::{Map, Value};

#[derive(Debug, Default)]
pub enum ScriptSrc {
    ScriptCode(String),
    #[default]
    ModuleName,
}
#[pyclass]
pub struct FunctionInput{
    #[pyo3(get, set)]
    data:PyObject,
}

#[derive(Debug, Default)]
pub struct PyScriptEntity {
    pub src: ScriptSrc,
    pub module_name: String,
    pub file_name: Option<String>,
    pub sys_path: Option<String>,
}

impl<S> From<S> for PyScriptEntity
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        Self::new(value, "default")
    }
}

impl PyScriptEntity {
    pub fn from_module<N: Into<String>>(module_name: N) -> Self {
        let src = ScriptSrc::ModuleName;
        let module_name = module_name.into();
        Self {
            src,
            module_name,
            ..Default::default()
        }
    }
    pub fn new<S: Into<String>, N: Into<String>>(src: S, module_name: N) -> Self {
        let src = ScriptSrc::ScriptCode(src.into());
        let module_name = module_name.into();
        Self {
            src,
            module_name,
            ..Default::default()
        }
    }
    pub fn set_sys_path<S: Into<String>>(mut self, path: S) -> Self {
        self.sys_path = Some(path.into());
        self
    }
    #[allow(dead_code)]
    pub fn set_file_name<S: Into<String>>(mut self, file_name: S) -> Self {
        self.file_name = Some(file_name.into());
        self
    }
    pub fn eval_function(&self, function_name: &str, args: Value) -> PyResult<Value> {
        wd_log::log_debug_ln!("eval_function -> {:?} args:{:?}" , self,args);
        let Self {
            src,
            module_name,
            file_name,
            sys_path,
        } = self;
        let file_name = file_name.clone().unwrap_or(format!("{}.py", module_name));
        Python::with_gil(move |py| {
            //设置系统path
            if let Some(path) = sys_path {
                let syspath: &PyList = py.import_bound("sys")?.getattr("path")?.extract()?;
                syspath.insert(0, &path)?;
            }
            //加载模型
            let module = match src {
                ScriptSrc::ScriptCode(script) => PyModule::from_code_bound(
                    py,
                    script.as_str(),
                    file_name.as_str(),
                    module_name.as_str(),
                )?,
                ScriptSrc::ModuleName => PyModule::import_bound(py, module_name.as_str())?,
            };
            //加载函数
            let function = module.getattr(function_name)?;
            //拼接输入
            let input_obj = Self::value_to_py_object(py, args)?;
            let input_class = FunctionInput{data :input_obj};
            //调起函数
            let output_obj = function.call((input_class,), None)?.extract::<PyObject>()?;
            //输出转格式
            let value = Self::py_object_to_value(output_obj, py)?;
            Ok(value)
        })
    }

    fn value_to_py_object(py: Python, value: Value) -> PyResult<PyObject> {
        match value {
            Value::Null => Ok(py.None()),
            Value::Bool(b) => Ok(b.into_py(py)),
            Value::Number(num) => {
                if let Some(n) = num.as_i64() {
                    Ok(n.into_py(py))
                } else if let Some(n) = num.as_u64() {
                    Ok(n.into_py(py))
                } else if let Some(n) = num.as_f64() {
                    Ok(n.into_py(py))
                } else {
                    unreachable!()
                }
            }
            Value::String(s) => Ok(s.into_py(py)),
            Value::Array(arr) => {
                let py_list = PyList::empty_bound(py);
                for item in arr {
                    py_list.append(Self::value_to_py_object(py, item)?)?;
                }
                Ok(py_list.into())
            }
            Value::Object(obj) => {
                let dict = PyDict::new_bound(py);
                for (key, value) in obj {
                    dict.set_item(key, Self::value_to_py_object(py, value)?)?;
                }
                Ok(dict.into())
            }
        }
    }

    fn py_object_to_value(obj: PyObject, py: Python) -> PyResult<Value> {
        // 如果 PyObject 是一个字典
        if let Ok(py_dict) = obj.downcast_bound::<PyDict>(py) {
            let mut map = Map::new();
            for (key, value) in py_dict.iter() {
                let key: String = key.extract()?;
                let json_value = Self::py_object_to_value(value.to_object(py), py)?;
                map.insert(key, json_value);
            }
            return Ok(Value::Object(map));
        }

        // 如果 PyObject 是一个列表
        if let Ok(py_list) = obj.downcast_bound::<PyList>(py) {
            let mut vec = Vec::new();
            for item in py_list.iter() {
                let json_value = Self::py_object_to_value(item.to_object(py), py)?;
                vec.push(json_value);
            }
            return Ok(Value::Array(vec));
        }

        // 如果 PyObject 是一个基础数据类型
        if let Ok(value) = obj.extract::<i64>(py) {
            return Ok(Value::Number(value.into()));
        }
        if let Ok(value) = obj.extract::<f64>(py) {
            return Ok(Value::Number(serde_json::Number::from_f64(value).unwrap()));
        }
        if let Ok(value) = obj.extract::<bool>(py) {
            return Ok(Value::Bool(value));
        }
        if let Ok(value) = obj.extract::<String>(py) {
            return Ok(Value::String(value));
        }

        // 如果 PyObject 是 None
        if obj.is_none(py) {
            return Ok(Value::Null);
        }

        // 如果 PyObject 是其他类型，你可能需要添加更多的处理逻辑
        // ...

        // 如果无法处理，则返回错误
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Unsupported PyObject type",
        ))
    }
}
#[cfg(test)]
mod test {
    use crate::py_runtime::py_script_code::PyScriptEntity;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Number, Value};

    const TEST_PYTHON_SCRIPT: &'static str = r#"
import sys

def handle(input):
    print("python =>",input)
    version = sys.version
    return {"code":input.data['code'],"msg":"success","version":version}
    "#;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Response {
        code: i32,
        msg: String,
        version: Option<String>,
    }

    #[test]
    fn test_eval_function() {
        let entity = PyScriptEntity::from(TEST_PYTHON_SCRIPT);

        let mut map = Map::new();
        map.insert("hello".into(), Value::String("world".into()));
        map.insert("code".into(), Value::Number(Number::from(1)));
        map.insert("obj".into(), Value::Null);

        let value = entity.eval_function("handle", Value::Object(map)).unwrap();
        let resp = serde_json::from_value::<Response>(value).unwrap();
        println!("--->{:?}", resp);
        assert_eq!(1, resp.code);
        assert_eq!("success", resp.msg);
        assert_eq!(true, resp.version.is_some());
    }

    #[test]
    fn test_eval_from_file() {
        let entity = PyScriptEntity::from_module("sys_info").set_sys_path("./custom_plugin");

        let value = entity
            .eval_function("get_system_info", Value::Null)
            .unwrap();
        println!("sys infra --->{}", value);
        let report = entity
            .eval_function("generate_system_report", value)
            .unwrap();
        println!("report ===> {}", report.to_string());
    }
}
