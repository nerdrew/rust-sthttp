use connection;
use mio;
use request;
use response;
use std;

const CONNECTIONS_START: mio::Token = mio::Token(0);

pub struct HttpClient<'a> {
    request_queue_tx: std::sync::mpsc::Sender<request::Request<'a>>,
}

impl<'a> HttpClient<'a> {
    pub fn new() -> HttpClient<'a> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut event_loop = mio::EventLoop::new().unwrap();
            let mut internal_client = InternalHttpClient {
                request_queue_rx: rx,
                connections: mio::util::Slab::new_starting_at(CONNECTIONS_START, 1024),
            };
            event_loop.run(&mut internal_client).unwrap();
        });

        return HttpClient {
            request_queue_tx: tx.clone(),
        }
    }
}

struct InternalHttpClient<'a> {
    request_queue_rx: std::sync::mpsc::Receiver<request::Request<'a>>,
    connections: mio::util::Slab<connection::Connection<'a>>,
}

impl<'a> mio::Handler for InternalHttpClient<'a> {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut mio::EventLoop<InternalHttpClient>, token: mio::Token, events: mio::EventSet) {
        debug!("{:?}: client connection is ready; events={:?}", token, events);
        self.connections[token].ready(events);

        match self.connections[token].state {
            connection::State::Reading => {
                event_loop.reregister(
                    &self.connections[token].stream,
                    token,
                    mio::EventSet::readable(),
                    mio::PollOpt::oneshot()).unwrap();
            },
            connection::State::Writing => {
                event_loop.reregister(
                    &self.connections[token].stream,
                    token,
                    mio::EventSet::writable(),
                    mio::PollOpt::oneshot()).unwrap();
            },
            connection::State::Closed => {
                let _ = self.connections.remove(token);
            }
        }
    }
}
