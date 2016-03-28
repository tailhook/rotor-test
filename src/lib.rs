extern crate rotor;
extern crate rotor_stream;

mod stream;
mod scope;

pub use stream::MemIo;
pub use scope::{MockLoop, Operation};
