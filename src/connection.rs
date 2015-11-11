use HttpServer;
use httparse;
use mio;
use mio::{TryRead, TryWrite};
use request;
use response;
use std;
use std::str::FromStr;

const MAX_HEADERS: usize = 50;

pub struct Connection<'a> {
    pub stream: mio::tcp::TcpStream,
    token: mio::Token,
    state: State,
    req_buf: Vec<u8>,
    req_headers: [httparse::Header<'a>; MAX_HEADERS],
    req_header_length: Option<usize>,
    req_content_length: Option<usize>,
    res_buf: std::io::Cursor<Vec<u8>>,
    handler: &'a super::Handler,
}

impl<'a> Connection<'a> {
    pub fn new(stream: mio::tcp::TcpStream, token: mio::Token, handler: &'a super::Handler) -> Connection<'a> {
        Connection {
            stream: stream,
            token: token,
            state: State::Reading,
            req_buf: Vec::with_capacity(4096),
            req_headers: [httparse::EMPTY_HEADER; MAX_HEADERS],
            req_header_length: None,
            req_content_length: None,
            res_buf: std::io::Cursor::new(Vec::with_capacity(4096)),
            handler: handler,
        }
    }

    pub fn ready(&mut self, event_loop: &mut mio::EventLoop<HttpServer>, events: mio::EventSet) {
        match self.state {
            State::Reading => {
                assert!(events.is_readable(), "unexpected events; events={:?}", events);
                self.read(event_loop)
            },
            State::Writing => {
                assert!(events.is_writable(), "unexpected events; events={:?}", events);
                self.write(event_loop)
            },
            _ => unimplemented!(),
        }
    }

    fn read(&mut self, event_loop: &mut mio::EventLoop<HttpServer>) {
        match self.stream.try_read_buf(&mut self.req_buf) {
            Ok(Some(0)) => {
                warn!("{:?}: read 0 bytes from client; buffered={}", self.token, self.req_buf.len());

                match self.req_buf.len() {
                    n if n > 0 => {
                        let mut res = response::Response::bad_request(Some("Incomplete request"));
                        res.finalize(self.res_buf.get_mut());
                        self.state = State::Writing;
                        self.reregister(event_loop);
                    }
                    _ => self.state = State::Closed,
                }
            }
            Ok(Some(n)) => {
                debug!(
                    "{:?}: read bytes={} buf={}", self.token, n,
                    unsafe { std::str::from_utf8_unchecked(&self.req_buf) });
                self.handle_request();
                self.reregister(event_loop);
            }
            Ok(None) => {
                self.reregister(event_loop);
            }
            Err(e) => {
                panic!("got an error trying to read; err={:?}", e);
            }
        }
    }

    fn write(&mut self, event_loop: &mut mio::EventLoop<HttpServer>) {
        match self.stream.try_write_buf(&mut self.res_buf) {
            Ok(Some(_)) => {
                if self.res_buf.position() >= self.res_buf.get_ref().len() as u64 {
                    debug!("finished writing response");
                    self.state = State::Reading;
                    self.res_buf.get_mut().clear();
                    self.res_buf.set_position(0);
                    self.req_buf.clear();
                }
            }
            Ok(None) => {
                debug!("not actually ready to write?");
            }
            Err(e) => {
                panic!("got an error trying to write; err={:?}", e);
            }
        }
        self.reregister(event_loop);
    }

    fn handle_request(&mut self) {
        let mut req = httparse::Request::new(&mut self.req_headers);

        if self.req_header_length.is_none() {
            match req.parse(&self.req_buf) {
                Ok(httparse::Status::Complete(offset)) => {
                    self.req_header_length = Some(offset);
                    self.req_content_length = Connection::get_content_length(&req.headers);
                },
                Ok(httparse::Status::Partial) => {
                    warn!("partial headers parsed");
                    return;
                },
                Err(e) => {
                    error!("parse error={:?}", e);
                    return;
                },
            }
        }

        match self.req_content_length {
            Some(content_length) => {
                if content_length + self.req_header_length.unwrap() > self.req_buf.len() {
                    debug!(
                        "Read complete headers, still waiting for body read={} content_length={}",
                        self.req_buf.len(),
                        self.req_content_length.expect("req_content_length must exist here"));
                    return;
                }
            },
            None => {
                debug!("No content length == no body");
            }
        }

        let mut res = response::Response::new();
        let body_start = self.req_header_length.unwrap();
        let body_end = body_start + self.req_content_length.unwrap_or(0);
        self.handler.handle(request::Request::new(&req, &self.req_buf[body_start..body_end]), &mut res);
        res.finalize(self.res_buf.get_mut());
        self.state = State::Writing;
    }

    fn get_content_length(headers: &[httparse::Header<'a>]) -> Option<usize> {
        for header in headers {
            if header.name == "Content-Length" {
                return usize::from_str(std::str::from_utf8(header.value).unwrap_or("")).ok()
            }
        }
        None
    }

    fn reregister(&self, event_loop: &mut mio::EventLoop<HttpServer>) {
        let event_set = match self.state {
            State::Reading => mio::EventSet::readable(),
            State::Writing => mio::EventSet::writable(),
            _ => mio::EventSet::none(),
        };

        event_loop.reregister(&self.stream, self.token, event_set, mio::PollOpt::oneshot())
            .unwrap();
    }

    pub fn is_closed(&self) -> bool {
        match self.state {
            State::Closed => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum State {
    Reading,
    Writing,
    Closed,
}
