use std::collections::VecDeque;
use poll_promise::Promise;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};

pub fn get(url: &str) -> Promise<anyhow::Result<Vec<u8>>> {
    let request = ehttp::Request::get(url);
    let (sender, receiver) = poll_promise::Promise::new();
    ehttp::fetch(request, |result| match result {
        Ok(o) => {
            sender.send(Ok(o.bytes));
        }
        Err(e) => sender.send(Err(anyhow::anyhow!("error:{}", e.to_string()))),
    });
    receiver
}
pub fn get_json(url: &str) -> HttpResponsePromise {
    let promise = get(url);
    HttpResponsePromise::Some(promise)
}

#[derive(Default)]
pub enum HttpResponsePromise {
    #[default]
    None,
    Some(Promise<anyhow::Result<Vec<u8>>>),
}
impl Debug for HttpResponsePromise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HttpJsonPromise[...]")
    }
}
impl HttpResponsePromise {
    pub fn try_get_json<T>(&mut self) -> Option<anyhow::Result<T>>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        let s = match self {
            HttpResponsePromise::None => return None,
            HttpResponsePromise::Some(s) => s,
        };
        if let Some(result) = s.ready() {
            let res = match result {
                Ok(o) => {
                    let t = match serde_json::from_slice::<T>(o) {
                        Ok(t) => t,
                        Err(e) => {
                            return Some(Err(anyhow::anyhow!(
                                "HttpJsonPromise unmarshal failed:{}",
                                e.to_string()
                            )))
                        }
                    };
                    Ok(t)
                }
                Err(e) => Err(anyhow::anyhow!("http get error:{}", e.to_string())),
            };
            *self = HttpResponsePromise::None;
            return Some(res);
        }
        None
    }
    pub fn try_get_string(&mut self) -> Option<anyhow::Result<String>>{
        let s = match self {
            HttpResponsePromise::None => return None,
            HttpResponsePromise::Some(s) => s,
        };
        if let Some(result) = s.ready() {
            let res = match result {
                Ok(o) => {
                    Ok(String::from_utf8_lossy(o.as_slice()).to_string())
                }
                Err(e) => Err(anyhow::anyhow!("http get error:{}", e.to_string())),
            };
            *self = HttpResponsePromise::None;
            return Some(res);
        }
        None
    }
}


pub fn post_json<B:serde::Serialize>(url:&str,body:&B,func:impl FnOnce(ehttp::Request)->ehttp::Request)-> anyhow::Result<HttpResponsePromise>
{
    let body = serde_json::to_vec(body)?;
    let request = ehttp::Request::post(url, body);
    let req = func(request);
    let (sender, receiver) = poll_promise::Promise::new();
    ehttp::fetch(req, |result| match result {
        Ok(o) => {
            sender.send(Ok(o.bytes));
        }
        Err(e) => sender.send(Err(anyhow::anyhow!("error:{}", e.to_string()))),
    });
    Ok(HttpResponsePromise::Some(receiver))
}

#[derive(Default,Debug,Clone)]
pub struct StreamResponse{
    status: bool,
    stream:Arc<Mutex<VecDeque<anyhow::Result<String>>>>
}

impl StreamResponse {
    pub fn is_over(&self)->bool {
        self.status
    }
    pub fn try_get_string(&mut self) -> Option<anyhow::Result<String>>{
        if self.status {
            return None
        }
        let mut lock = self.stream.lock().unwrap();
        if let Some(res) = lock.pop_front() {
            if let Ok(ref s) = res {
                if s == "->over<-"{
                    self.status = true;
                    return None
                }
            }
            Some(res)
        }else{
            None
        }
    }
    pub fn try_get_obj<T>(&mut self) -> Option<anyhow::Result<T>>
        where
            T: for<'a> serde::Deserialize<'a>,
    {
        if self.status {
            return None
        }
        let mut lock = self.stream.lock().unwrap();
        if let Some(res) = lock.pop_front() {
            if let Ok(ref s) = res {
                if s == "->over<-"{
                    self.status = true;
                    return None
                }
                match serde_json::from_slice::<T>(s.as_bytes()) {
                    Ok(o)=>{
                        return Some(Ok(o))
                    }
                    Err(e)=>{
                        return Some(Err(anyhow::anyhow!("{}",e.to_string())))
                    }
                }
            }
            if let Err(e)= res{
                return Some(Err(anyhow::anyhow!("{}",e.to_string())))
            }
            None
        }else{
            None
        }
    }
}

pub fn post_json_stream<B:serde::Serialize>(url:&str,body:&B,func:impl FnOnce(ehttp::Request)->ehttp::Request)-> anyhow::Result<StreamResponse>
{
    let body = serde_json::to_vec(body)?;
    let request = ehttp::Request::post(url, body);
    let req = func(request);

    let recv = StreamResponse::default();
    let send = recv.stream.clone();

    ehttp::streaming::fetch (req, move |result| {
        let part = match result {
            Ok(part) => part,
            Err(err) => {
                let mut lock = send.lock().unwrap();
                lock.push_back(Err(anyhow::anyhow!("{}",err.to_string())));
                return std::ops::ControlFlow::Break(());
            }
        };

        match part {
            ehttp::streaming::Part::Response(response) => {
                if response.ok {
                    std::ops::ControlFlow::Continue(())
                } else {
                    let mut lock = send.lock().unwrap();
                    lock.push_back(Ok("->over<-".to_string()));
                    std::ops::ControlFlow::Break(())
                }
            }
            ehttp::streaming::Part::Chunk(chunk) => {
                let mut lock = send.lock().unwrap();
                if chunk.is_empty() {
                    lock.push_back(Ok("->over<-".to_string()));
                    std::ops::ControlFlow::Break(())
                }else{
                    let s = String::from_utf8_lossy(chunk.as_slice()).to_string();
                    lock.push_back(Ok(s));
                    std::ops::ControlFlow::Continue(())
                }

            }
        }

    });
    Ok(recv)
}

#[cfg(test)]
mod test {
    // use crate::infra::get;

    // #[test]
    // pub fn test_get(){
    //     let resp = get("http://127.0.0.1:50000/api/v1/plugin");
    //     println!("--->{:?}",resp)
    // }
}
