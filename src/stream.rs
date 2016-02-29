use std::io;
use std::fmt;
use std::cmp::min;
use std::sync::{Arc, Mutex, MutexGuard};

use rotor::mio;

/// In memory stream
///
/// The struct pretends to be `mio::Evented` but it have `unimplemented` panic
/// when actually added to the loop. I.e. it should be used in tests which
/// use plain state machine, and not the event loop.
///
/// Clarification: it implements `Read`/`Write` but, it's not a pipe. I.e.
/// buffers for `Read` and `Write` are separate. You use `push_xxx` methods to
/// add data for the next `Read::read`.
///
/// You should clone the stream. Feed one to the application and second one
/// to the unit testing code.
#[derive(Clone)]
pub struct MemIo(Arc<Mutex<Bufs>>);

struct Bufs {
    input: Vec<u8>,
    input_closed: bool,
    output: Vec<u8>,
}

impl MemIo {
    /// Create a stream
    ///
    /// Stream start empty
    pub fn new() -> MemIo {
        MemIo(Arc::new(Mutex::new(Bufs {
            input: Vec::new(),
            input_closed: false,
            output: Vec::new(),
        })))
    }
    /// Push some bytes to an input buffer of an application
    pub fn push_bytes<T:AsRef<[u8]>>(&mut self, val: T) {
        let mut bufs = self.bufs();
        bufs.input.extend(val.as_ref());
        assert!(!bufs.input_closed);
    }
    /// Marks input as closed so application gets end-of-stream event on next
    /// read
    pub fn shutdown_input(&self) {
        self.bufs().input_closed = true;
    }
    /// Get output as a string
    ///
    /// This is created by `String::from_utf8_lossy` so kinda works for binary
    /// data too, but not exactly. If you need to compare some non-textual
    /// data use `output_bytes()`
    ///
    /// The data in the buffer are not discarded. Next call will return same
    /// data again.
    pub fn output_str(&self) -> String {
        // Unfortunately we can't return a slice, because of borrowing rules
        // but it's for unit tests, so we don't care performance
        String::from_utf8_lossy(&self.bufs().output).to_string()
    }
    /// Get data in the output buffer
    ///
    /// The data in the buffer are not discarded. Next call will return same
    /// data again.
    pub fn output_bytes(&self) -> Vec<u8> {
        // Unfortunately we can't return a slice, because of borrowing rules
        // but it's for unit tests, so we don't care performance
        self.bufs().output.clone()
    }
    fn bufs(&self) -> MutexGuard<Bufs> {
        self.0.lock().expect("Poisoned MemIo (mock stream)")
    }
}

impl fmt::Debug for MemIo {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let bufs = self.bufs();
        fmt.debug_struct("MemIo")
        .field("input", &String::from_utf8_lossy(&bufs.input))
        .field("input_closed", &bufs.input_closed)
        .field("output", &String::from_utf8_lossy(&bufs.output))
        .finish()
    }
}

impl io::Read for MemIo {
    fn read(&mut self, val: &mut [u8]) -> io::Result<usize> {
        let mut bufs = self.bufs();
        let bytes = min(val.len(), bufs.input.len());
        if bytes > 0 {
            assert_eq!(io::copy(
                &mut io::Cursor::new(&bufs.input),
                &mut io::Cursor::new(val))
                .expect("copy always work"), bytes as u64);
            bufs.input.drain(..bytes);
            Ok(bytes)
        } else {
            if bufs.input_closed {
                Ok(0)
            } else {
                Err(io::Error::new(io::ErrorKind::WouldBlock,
                    "no data in mocked input buffer"))
            }
        }
    }
}
impl io::Write for MemIo {
    fn write(&mut self, val: &[u8]) -> io::Result<usize> {
        let mut bufs = self.bufs();
        io::copy(&mut io::Cursor::new(val), &mut bufs.output)
            .map(|x| x as usize)
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl mio::Evented for MemIo {
    fn register(&self, _selector: &mut mio::Selector,
        _token: mio::Token, _interest: mio::EventSet, _opts: mio::PollOpt)
        -> io::Result<()>
    { unreachable!("trying to poll on mock stream") }
    fn reregister(&self, _selector: &mut mio::Selector, _token: mio::Token,
        _interest: mio::EventSet, _opts: mio::PollOpt) -> io::Result<()>
    { unreachable!("trying to poll on mock stream") }
    fn deregister(&self, _selector: &mut mio::Selector) -> io::Result<()>
    { unreachable!("trying to poll on mock stream") }
}

#[cfg(test)]
mod self_test {
    use std::io::{Read, Write};
    use super::MemIo;

    #[test]
    fn input() {
        let mut s = MemIo::new();
        s.push_bytes("hello world");
        s.shutdown_input();
        let mut b = String::new();
        assert_eq!(s.read_to_string(&mut b).unwrap(), 11);
        assert_eq!(&b, "hello world");
    }

    #[test]
    fn output() {
        let mut s = MemIo::new();
        s.write(b"hello").expect("write failed");
        s.write(b"world").expect("write failed");
        assert_eq!(s.output_str(), "helloworld");
    }

}
