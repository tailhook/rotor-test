use std::io;

use rotor::mio;
use rotor::{Scope, Time, PollOpt, EventSet};
use rotor::{_scope, _Timeo, _Notify, _LoopApi};

/// Operation that was done with Scope
#[derive(Debug, PartialEq, Eq)]
pub enum Operation {
    Register(EventSet, PollOpt),
    Reregister(EventSet, PollOpt),
    Deregister,
    Shutdown,
}

struct Handler {
    operations: Vec<Operation>,
}

/// A mock loop implementation
///
/// It's not actually fully working loop, just a thing which you can get
/// a `Scope` object from.
pub struct MockLoop<C> {
    event_loop: mio::EventLoop<Handler>,
    handler: Handler,
    context: C,
    channel: mio::Sender<_Notify>,
}

impl<C> MockLoop<C> {
    /// Create a mock loop
    ///
    /// The `ctx` is a context, and it's type must be compatible
    /// to your state machine.
    pub fn new(ctx: C) -> MockLoop<C> {
        let eloop = mio::EventLoop::new()
                .expect("event loop is crated");
        MockLoop {
            handler: Handler {
                operations: Vec::new(),
            },
            channel: eloop.channel(),
            event_loop: eloop,
            context: ctx,
        }
    }
    /// Get a scope object for specified token
    ///
    /// This is useful to call state machine actions directly
    pub fn scope(&mut self, x: usize) -> Scope<C> {
        _scope(Time::zero(), mio::Token(x),
            &mut self.context,
            &mut self.channel,
            &mut self.handler)
    }

    pub fn ctx(&mut self) -> &mut C {
        &mut self.context
    }
}

impl mio::Handler for Handler {
    type Timeout = _Timeo;
    type Message = _Notify;
}

impl _LoopApi for Handler
{
    fn register(&mut self, _io: &mio::Evented, _token: mio::Token,
        interest: EventSet, opt: PollOpt) -> io::Result<()>
    {
        self.operations.push(Operation::Register(interest, opt));
        Ok(())
    }

    fn reregister(&mut self, _io: &mio::Evented, _token: mio::Token,
        interest: EventSet, opt: PollOpt) -> io::Result<()>
    {
        self.operations.push(Operation::Reregister(interest, opt));
        Ok(())
    }

    fn deregister(&mut self, _io: &mio::Evented) -> io::Result<()>
    {
        self.operations.push(Operation::Deregister);
        Ok(())
    }

    fn timeout_ms(&mut self, _token: mio::Token, _delay: u64)
        -> Result<mio::Timeout, mio::TimerError>
    {
        panic!("Deprecated API");
    }
    fn clear_timeout(&mut self, _token: mio::Timeout) -> bool
    {
        panic!("Deprecated API");
    }
    fn shutdown(&mut self) {
        self.operations.push(Operation::Shutdown);
    }
}

#[cfg(test)]
mod self_test {

    use rotor::{Machine, EventSet, Scope, Response};
    use rotor::void::{unreachable, Void};
    use super::MockLoop;

    #[derive(PartialEq, Eq, Debug)]
    struct M(u32);


    impl Machine for M {
        type Context = ();
        type Seed = Void;
        fn create(seed: Self::Seed, _scope: &mut Scope<()>)
            -> Response<Self, Void>
        {
            unreachable(seed)
        }
        fn ready(self, _events: EventSet, _scope: &mut Scope<()>)
            -> Response<Self, Self::Seed>
        {
            unimplemented!();
        }
        fn spawned(self, _scope: &mut Scope<()>) -> Response<Self, Self::Seed>
        {
            unimplemented!();
        }
        fn timeout(self, _scope: &mut Scope<()>) -> Response<Self, Self::Seed>
        {
            unimplemented!();
        }
        fn wakeup(self, _scope: &mut Scope<()>) -> Response<Self, Self::Seed>
        {
            Response::ok(M(self.0 + 1))
        }
    }

    #[test]
    fn test_machine() {
        let mut factory = MockLoop::new(());
        let m = M(10);
        let mut value = None;
        Machine::wakeup(m, &mut factory.scope(1)).wrap(|x| value = Some(x));
        assert_eq!(value, Some(M(11)));
    }
}
