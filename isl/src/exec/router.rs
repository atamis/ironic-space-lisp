//! Contains router runtime code including channels,
//! valid router messages, and the router spawning function.
use crate::data;
use crate::data::Literal;
use crate::futures::StreamExt;
use futures::channel::mpsc;
use futures::future::select;
use futures::future::Either;
use petgraph::graphmap::DiGraphMap;
use petgraph::Direction;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time;

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
    /// Establish a one way watch between the first and the second pid so that
    /// the first pid is informed when the second exits.
    Watch(data::Pid, data::Pid),
    /// Safely close the router once all other handlers are dropped..
    Quit,
}

struct Router {
    rx: mpsc::Receiver<RouterMessage>,
    queue: VecDeque<RouterMessage>,
    // Map of watchers -> watched
    watches: DiGraphMap<data::Pid, ()>,
    state: RouterState,
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
            } else if self.is_done() {
                // 2s timeout of no messages before quiting
                let t = time::delay_for(Duration::from_millis(2000));

                match select(self.rx.next(), t).await {
                    Either::Left((m, _)) => m,
                    Either::Right((_, _)) => break,
                }
            } else {
                self.rx.next().await
            };

            if self.debug {
                println!("Recieved message {:?}", m);
            }

            match m {
                None => break,
                Some(RouterMessage::Close(p)) => self.close(p),
                Some(RouterMessage::Register(p, tx)) => self.register(p, tx),
                Some(RouterMessage::Send(p, l)) => self.send(p, l),
                Some(RouterMessage::Watch(p1, p2)) => self.watch(p1, p2),
                Some(RouterMessage::Quit) => self.quit(),
            };
        }

        if self.debug {
            println!(
                "Router finished (quitting: {:?}): {:?}",
                self.quitting, self.state
            );
        }
    }

    fn close(&mut self, p: data::Pid) {
        self.state.remove(&p);
        for watcher in self.watches.neighbors_directed(p, Direction::Incoming) {
            println!("Found that {:?} watched {:?} die", watcher, p);
            self.queue.push_back(RouterMessage::Send(
                watcher,
                vector![data::Literal::Keyword("exit".into()), p.into()].into(),
            ))
        }
    }

    fn register(&mut self, p: data::Pid, tx: mpsc::Sender<Literal>) {
        self.state.insert(p, tx);
    }

    fn send(&mut self, p: data::Pid, l: data::Literal) {
        if let Some(chan) = self.state.get_mut(&p) {
            if let Err(e) = chan.try_send(l) {
                eprintln!(
                    "Attempted to send on closed channel {:?}, but encountered error: {:?}",
                    p, e
                );
                self.state.remove(&p);
            }
        } else {
            eprintln!("Attempted to send to non-existant pid {:?}: {:?}", p, l)
        }
    }

    fn watch(&mut self, watcher: data::Pid, watched: data::Pid) {
        self.watches.add_edge(watcher, watched, ());
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
