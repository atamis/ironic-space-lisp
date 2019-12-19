 use tokio::runtime::Runtime;
use crate::data::Literal;
use crate::data;
use futures::future::Future;
use std::collections::HashMap;
use futures::channel::mpsc;
use futures::future::select;
use futures::future::Either;
use crate::futures::StreamExt;
use std::collections::VecDeque;
use std::time::Duration;
use tokio_timer;
use petgraph::graphmap::DiGraphMap;

/// A channel to the message router.
pub type RouterChan = mpsc::Sender<RouterMessage>;
type RouterState = HashMap<data::Pid, mpsc::Sender<Literal>>;

/// Messages you can send to the router.
#[derive(Debug)]
pub enum RouterMessage {
    /// Deregister a Pid.
    Close(data::Pid),
    /// Register a Pid with a Sender channel.
    Register(data::Pid, mpsc::Sender<Literal>),
    /// Send some data to the channel associated with a Pid.
    Send(data::Pid, Literal),
    /// Safely close the router once all other handlers are dropped..
    Quit,
}

struct Router {
    rx: mpsc::Receiver<RouterMessage>,
    queue: VecDeque<RouterMessage>,
    watches: DiGraphMap<data::Pid, ()>,
    state:  RouterState,
    quitting: bool,
    debug: bool,
}

impl Router {
    fn new(rx: mpsc::Receiver<RouterMessage>) -> Router {
        Router {
            rx,
            queue: VecDeque::new(),
            state: RouterState::new(),
            watches: DiGraphMap::new(),
            quitting: false,
            debug: false,
        }
    }

    async fn run(mut self) {
        if self.debug {
            println!("Router starting");
        }
        loop {
            let m = if let Some(m) = self.queue.pop_front() {
                Some(m)
            } else {
                if self.is_done() {
                    // 2s timeout of no messages before quiting
                    let t = tokio_timer::delay_for(Duration::from_millis(2000));

                    match select(self.rx.next(), t).await {
                        Either::Left((m, _)) => m,
                        Either::Right((_, _)) => break,
                    }
                } else {
                    self.rx.next().await
                }
            };

            if self.debug {
                println!("Recieved message {:?}", m);
            }

            match m {
                None => break,
                Some(RouterMessage::Close(p)) => self.close(p),
                Some(RouterMessage::Register(p, tx)) => self.register(p, tx) ,
                Some(RouterMessage::Send(p, l)) => self.send(p, l),
                Some(RouterMessage::Quit) => self.quit(),
            };
        }

        if self.debug {
            println!("Router finished (quitting: {:?}): {:?}", self.quitting, self.state);
        }
    }

    fn close(&mut self, p: data::Pid) {
        self.state.remove(&p);
    }

    fn register(&mut self, p: data::Pid, tx: mpsc::Sender<Literal>) {
        self.state.insert(p, tx);
    }

    fn send(&mut self, p: data::Pid, l: data::Literal) {
        if let Some(chan) = self.state.get_mut(&p) {
            if let Err(e) = chan.try_send(l) {
                eprintln!("Attempted to send on closed channel {:?}, but encountered error: {:?}", p, e);
                self.state.remove(&p);
            }
        } else {
            eprintln!("Attempted to send to non-existant pid {:?}: {:?}", p, l)
        }
    }

    fn quit(&mut self) {
        self.quitting = true;
    }

    fn is_done(&mut self) -> bool {
        self.quitting && self.state.is_empty() && self.queue.is_empty()
    }
}

/// Spawn a router on the runtime.
///
/// Routers respond to router messages sent on the sender channel this function returns.
pub fn router(runtime: &mut Runtime) -> mpsc::Sender<RouterMessage> {
    let (tx, rx) = mpsc::channel::<RouterMessage>(10);

    let f = Router::new(rx).run();

    runtime.spawn(f);

    tx
}
