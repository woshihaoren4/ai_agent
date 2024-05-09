use std::any::Any;

const ARGS_DEFAULT:&'static str = "";

#[derive(Debug, Default)]
pub struct Input {
    input: String,
}

#[derive(Debug,Default)]
pub struct Output {
    any: Box<dyn Any>,
}
impl Output{
    pub fn null()->Self{
        Self::new(())
    }
    pub fn new<T:Any>(t:T)->Self{
        Output{any:Box::new(t)}
    }
}
