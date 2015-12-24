use httparse;
use status;

const INITIAL_BODY_SIZE: usize = 4096;
const INITIAL_HEADER_COUNT: usize = 50;

pub struct Request<'a> {
    pub method: Option<&'a str>,
    pub path: Option<&'a str>,
    pub headers: Vec<httparse::Header<'a>>,
    pub body: Vec<u8>,
}

impl<'a> Request<'a> {
    pub fn from_httparse(req: &'a httparse::Request, body: &'a [u8]) -> Request<'a> {
        Request {
            method: req.method,
            path: req.path,
            headers: req.headers.to_vec(),
            body: body.to_vec(),
        }
    }

    pub fn new() -> Request<'a> {
        Request {
            method: None,
            path: None,
            headers: Vec::with_capacity(INITIAL_HEADER_COUNT),
            body: Vec::with_capacity(INITIAL_BODY_SIZE),
        }
    }

    // TODO validate header
    pub fn add_header(&mut self, name: &'a str, value: &'a [u8]) {
        self.headers.push(httparse::Header { name: name, value: value});
    }

    pub fn add_body(&mut self, body: &'a [u8]) {
        self.body.extend_from_slice(body);
    }

    pub fn finalize(&mut self, buf: &mut Vec<u8>) {
        //if self.method.is_none() {
            //self.code = Some(status::Code::InternalServerError500);
        //}

        //let preamble = format!(
            //"{} {}\r\nContent-Length: {}\r\n",
            //self.method.as_ref().expect("all requests have a method"),
            //self.path.as_ref().expect("all requests have a path"),
            //self.body.len());

        //buf.append(&mut preamble.into_bytes());

        //for header in self.headers.iter() {
            //buf.extend_from_slice(header.name.as_bytes());
            //buf.extend_from_slice(b": ");
            //buf.extend_from_slice(header.value);
            //buf.extend_from_slice(b"\r\n");
        //}
        //buf.extend_from_slice(b"\r\n");

        //buf.append(&mut self.body);
    }
}
