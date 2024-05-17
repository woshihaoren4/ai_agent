use std::any::Any;

#[derive(Debug)]
pub struct Output {
    pub raw_to_ctx: bool,
    pub any: Box<dyn Any + Send + Sync + 'static>,
}
impl Default for Output {
    fn default() -> Self {
        let raw_to_ctx = false;
        Self {
            raw_to_ctx,
            any: Box::new(()),
        }
    }
}
// impl<T> TryInto<T> for Output{
//     type Error = anyhow::Error;
//
//     fn try_into(self) -> Result<T, Self::Error> {
//         let val:Box<T> = self.any.downcast()?;
//         Ok(*val)
//     }
// }
impl Output {
    pub fn null() -> Self {
        Self::new(())
    }
    pub fn new<T: Any + Send + Sync + 'static>(t: T) -> Self {
        Output {
            any: Box::new(t),
            ..Default::default()
        }
    }
    pub fn raw_to_ctx(mut self) -> Self {
        self.raw_to_ctx = true;
        self
    }
    pub fn try_into<T: 'static>(self) -> Option<T> {
        if let Ok(s) = self.any.downcast() {
            Some(*s)
        } else {
            None
        }
    }
    pub fn into<T: Default + 'static>(self) -> T {
        self.try_into().unwrap_or_default()
    }
}
