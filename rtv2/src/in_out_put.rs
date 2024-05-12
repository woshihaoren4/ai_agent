use std::any::Any;

#[derive(Debug, Default)]
pub struct Input {
    input: String,
}

#[derive(Debug)]
pub struct Output {
    pub raw_to_ctx:bool,
    pub any: Box<dyn Any + Send + Sync + 'static>,
}
impl Default for Output{
    fn default() -> Self {
        let raw_to_ctx = false;
        Self{ raw_to_ctx,any:Box::new(())}
    }
}
impl Output{
    pub fn null()->Self{
        Self::new(())
    }
    pub fn new<T:Any+ Send + Sync + 'static>(t:T)->Self{
        Output{any:Box::new(t),..Default::default()}
    }
    pub fn raw_to_ctx(mut self)-> Self{
        self.raw_to_ctx = true;self
    }
}
