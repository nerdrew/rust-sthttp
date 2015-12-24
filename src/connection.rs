use httparse;
use mio;
use mio::{TryRead, TryWrite};
use request;
use response;
use server;
use std;
use std::str::FromStr;

const MAX_HEADERS: usize = 50;

pub struct Connection<'a> {
    handler: &'a server::Handler,
    incoming_buf: Vec<u8>,
    incoming_content_length: Option<usize>,
    incoming_header_length: Option<usize>,
    incoming_headers: [httparse::Header<'a>; MAX_HEADERS],
    outgoing_buf: std::io::Cursor<Vec<u8>>,
    pub state: State,
    pub stream: mio::tcp::TcpStream,
    token: mio::Token,
}

impl<'a> Connection<'a> {
    pub fn server(stream: mio::tcp::TcpStream, token: mio::Token, handler: &'a server::Handler) -> Connection<'a> {
        Connection {
            stream: stream,
            handler: handler,
            incoming_buf: Vec::with_capacity(4096),
            incoming_content_length: None,
            incoming_header_length: None,
            incoming_headers: [httparse::EMPTY_HEADER; MAX_HEADERS],
            outgoing_buf: std::io::Cursor::new(Vec::with_capacity(4096)),
            state: State::Reading,
            token: token,
        }
    }

    pub fn ready(&mut self, events: mio::EventSet) {
        match self.state {
            State::Reading => {
                assert!(events.is_readable(), "unexpected events; events={:?}", events);
                self.read()
            },
            State::Writing => {
                assert!(events.is_writable(), "unexpected events; events={:?}", events);
                self.write()
            },
            _ => unimplemented!(),
        };
    }

    fn read(&mut self) {
        match self.stream.try_read_buf(&mut self.incoming_buf) {
            Ok(Some(0)) => {
                warn!("{:?}: read 0 bytes from client; buffered={}", self.token, self.incoming_buf.len());

                match self.incoming_buf.len() {
                    n if n > 0 => {
                        let mut res = response::Response::bad_request(Some("Incomplete request"));
                        res.finalize(self.outgoing_buf.get_mut());
                        self.state = State::Writing;
                    }
                    _ => self.state = State::Closed,
                }
            }
            Ok(Some(n)) => {
                debug!(
                    "{:?}: read bytes={} buf={}", self.token, n,
                    unsafe { std::str::from_utf8_unchecked(&self.incoming_buf) });
                self.parse_request();
            }
            Ok(None) => {}
            Err(e) => {
                error!("got an error trying to read; err={:?}", e);
                self.state = State::Closed;
            }
        }
    }

    fn write(&mut self) {
        match self.stream.try_write_buf(&mut self.outgoing_buf) {
            Ok(Some(_)) => {
                if self.outgoing_buf.position() >= self.outgoing_buf.get_ref().len() as u64 {
                    debug!("finished writing response");
                    self.state = State::Reading;
                    self.outgoing_buf.get_mut().clear();
                    self.outgoing_buf.set_position(0);
                    self.incoming_buf.clear();
                }
            }
            Ok(None) => {
                debug!("not actually ready to write?");
            }
            Err(e) => {
                panic!("got an error trying to write; err={:?}", e);
            }
        }
    }

    fn parse_request(&mut self) {
        let mut req = httparse::Request::new(&mut self.incoming_headers);

        if self.incoming_header_length.is_none() {
            match req.parse(&self.incoming_buf) {
                Ok(httparse::Status::Complete(offset)) => {
                    self.incoming_header_length = Some(offset);
                    self.incoming_content_length = Connection::get_content_length(&req.headers);
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

        match self.incoming_content_length {
            Some(content_length) => {
                if content_length + self.incoming_header_length.unwrap() > self.incoming_buf.len() {
                    debug!(
                        "Read complete headers, still waiting for body read={} content_length={}",
                        self.incoming_buf.len(), content_length);
                    return;
                }
            },
            None => {
                debug!("No content length == no body");
            }
        }

        let mut res = response::Response::new();
        let body_start = self.incoming_header_length.unwrap();
        let body_end = body_start + self.incoming_content_length.unwrap_or(0);
        self.handler.handle(request::Request::from_httparse(&req, &self.incoming_buf[body_start..body_end]), &mut res);
        res.finalize(self.outgoing_buf.get_mut());
        self.state = State::Writing;
    }

    fn parse_response(&mut self) {
        return;
        //let mut res = httparse::Response::new(&mut self.incoming_headers);

        //if self.incoming_header_length.is_none() {
            //match res.parse(&self.incoming_buf) {
                //Ok(httparse::Status::Complete(offset)) => {
                    //self.incoming_header_length = Some(offset);
                    //self.incoming_content_length = Connection::get_content_length(&res.headers);
                //},
                //Ok(httparse::Status::Partial) => {
                    //warn!("partial headers parsed");
                    //return;
                //},
                //Err(e) => {
                    //error!("parse error={:?}", e);
                    //return;
                //},
            //}
        //}

        //match self.incoming_content_length {
            //Some(content_length) => {
                //if content_length + self.incoming_header_length.unwrap() > self.incoming_buf.len() {
                    //debug!(
                        //"Read complete headers, still waiting for body read={} content_length={}",
                        //self.incoming_buf.len(), content_length);
                    //return;
                //}
            //},
            //None => {
                //debug!("No content length == no body");
            //}
        //}

        //let body_start = self.incoming_header_length.unwrap();
        //let body_end = body_start + self.incoming_content_length.unwrap_or(0);
        //self.callback.unwrap().run(response::Response::new(&res, &self.incoming_buf[body_start..body_end]));
        //self.state = State::Writing;
    }

    // TODO: handle Transfer-Encoding
    pub fn get_content_length(headers: &[httparse::Header<'a>]) -> Option<usize> {
        for header in headers {
            if header.name == "Content-Length" {
                return usize::from_str(std::str::from_utf8(header.value).unwrap_or("")).ok()
            }
        }
        None
    }
}

#[derive(Debug)]
pub enum State {
    Reading,
    Writing,
    Closed,
}
