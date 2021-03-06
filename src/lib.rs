#![feature(vec_push_all)]

#[macro_use] extern crate log;
extern crate bytes;
extern crate env_logger;
extern crate httparse;
extern crate mio;
extern crate time;

mod connection;
pub mod request;
pub mod response;
pub mod status;

const SERVER: mio::Token = mio::Token(0);
const CONNECTIONS_START: mio::Token = mio::Token(1);

/// A handler that can handle incoming requests for a server.
pub trait Handler: Sync + Send {
    /// Receives a `Request`/`Response` pair, and should perform some action on them.
    ///
    /// This could reading from the request, and writing to the response.
    fn handle<'a>(&self, request::Request<'a>, &mut response::Response);

    /// Called when a Request includes a `Expect: 100-continue` header.
    ///
    /// By default, this will always immediately response with a `StatusCode::Continue`,
    /// but can be overridden with custom behavior.
    //fn check_continue(&self, _: (&Method, &RequestUri, &Headers)) -> StatusCode {
        //StatusCode::Continue
    //}

    /// This is run after a connection is received, on a per-connection basis (not a
    /// per-request basis, as a connection with keep-alive may handle multiple
    /// requests)
    fn on_connection_start(&self) { }

    /// This is run before a connection is closed, on a per-connection basis (not a
    /// per-request basis, as a connection with keep-alive may handle multiple
    /// requests)
    fn on_connection_end(&self) { }
}

impl<F> Handler for F where F: Fn(request::Request, &mut response::Response), F: Sync + Send {
    fn handle<'a>(&self, req: request::Request<'a>, res: &mut response::Response) {
        self(req,res)
    }
}

pub struct HttpServer<'a> {
    server: mio::tcp::TcpListener,
    connections: mio::util::Slab<connection::Connection<'a>>,
    handler: &'a Handler,
}

impl<'a> HttpServer<'a> {
    pub fn new(server: mio::tcp::TcpListener, handler: &'a Handler) -> HttpServer<'a> {
        HttpServer {
            server: server,
            connections: mio::util::Slab::new_starting_at(CONNECTIONS_START, 1024),
            handler: handler,
        }
    }
}

impl<'a> mio::Handler for HttpServer<'a> {
    type Timeout = (); // Timeouts are not used in this example
    type Message = (); // Cross thread notifications are not used in this example

    fn ready(&mut self, event_loop: &mut mio::EventLoop<HttpServer>, token: mio::Token, events: mio::EventSet) {
        match token {
            SERVER => {
                assert!(events.is_readable());

                match self.server.accept() {
                    Ok(Some(stream)) => {
                        debug!("accepted a new connection");

                        let handler = self.handler;
                        let token = self.connections.insert_with(|token| {
                            connection::Connection::new(stream, token, handler)
                        }).unwrap();

                        event_loop.register_opt(
                            &self.connections[token].stream,
                            token,
                            mio::EventSet::readable(),
                            mio::PollOpt::oneshot()).unwrap();
                    }
                    Ok(None) => {
                        error!("the server stream wasn't actually ready");
                    }
                    Err(e) => {
                        error!("encountered error while accepting connection; err={:?}", e);
                        event_loop.shutdown();
                    }
                }
            }
            _ => {
                debug!("{:?}: connection is ready; events={:?}", token, events);
                self.connections[token].ready(event_loop, events);

                if self.connections[token].is_closed() {
                    let _ = self.connections.remove(token);
                }
            }
        }
    }
}

pub fn start(address: std::net::SocketAddr, handler: &Handler) {
    let server = mio::tcp::TcpListener::bind(&address).unwrap();
    let mut event_loop = mio::EventLoop::new().unwrap();
    debug!("Listening for HTTP on {}", address);
    event_loop.register_opt(&server, SERVER, mio::EventSet::readable(), mio::PollOpt::empty()).unwrap();
    let mut http_server = HttpServer::new(server, handler);
    event_loop.run(&mut http_server).unwrap();
}
