use std::fmt::{Debug, Formatter};
use poll_promise::Promise;

pub fn get(url:&str) -> Promise<anyhow::Result<Vec<u8>>>{
    let request = ehttp::Request::get(url);
    let (sender, receiver) = poll_promise::Promise::new();
    ehttp::fetch(request,|result|{
        match result {
            Ok(o) => {
                sender.send(Ok(o.bytes));
            }
            Err(e) => {
                sender.send(Err(anyhow::anyhow!("error:{}",e.to_string())))
            }
        }
    });
    receiver
}
pub fn get_json(url:&str) -> HttpJsonPromise {
    let promise = get(url);
    HttpJsonPromise::Some(promise)
}
#[derive(Default)]
pub enum HttpJsonPromise{
    #[default]
    None,
    Some(Promise<anyhow::Result<Vec<u8>>>),
}
impl Debug for HttpJsonPromise{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"HttpJsonPromise[...]")
    }
}
impl HttpJsonPromise{
    pub fn try_get_value<T>(&mut self) ->Option<anyhow::Result<T>>
    where T: for<'a> serde::Deserialize<'a>
    {
        let s = match self {
            HttpJsonPromise::None => return None,
            HttpJsonPromise::Some(s) => s,
        };
        if let Some(result) = s.ready() {
            let res = match result {
                Ok(o) => {
                    let t = match serde_json::from_slice::<T>(o) {
                        Ok(t)=>t,
                        Err(e)=>return Some(Err(anyhow::anyhow!("HttpJsonPromise unmarshal failed:{}",e.to_string())))

                    };
                    Ok(t)
                }
                Err(e) => Err(anyhow::anyhow!("http get error:{}",e.to_string())),
            };
            *self = HttpJsonPromise::None;
            return Some(res)
        }
        None
    }
}

// pub fn get_json<S>(url:&str)->anyhow::Result<S>
// where S: for <'a> serde::Deserialize<'a>
// {
//     let data = get(url)?;
//     let s = serde_json::from_slice(data.as_slice())?;Ok(s)
// }


#[cfg(test)]
mod test{
    // use crate::infra::get;

    // #[test]
    // pub fn test_get(){
    //     let resp = get("http://127.0.0.1:50000/api/v1/plugin");
    //     println!("--->{:?}",resp)
    // }
}