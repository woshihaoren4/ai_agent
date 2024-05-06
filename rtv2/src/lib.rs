mod context;
mod in_out_put;
mod task;
mod define;
mod runtime;
mod error;

pub use context::*;
pub use in_out_put::*;
pub use task::*;
pub use define::*;
pub use runtime::*;
pub use error::*;

#[cfg(test)]
mod tests {

    #[test]
    pub fn test_hello(){
        println!("hello world")
    }
}
