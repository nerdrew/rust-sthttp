use httparse;
use status;
use time;

const INITIAL_BODY_SIZE: usize = 4096;
const INITIAL_HEADER_COUNT: usize = 50;

pub struct Response<'a> {
    code: Option<status::Code>,
    headers: Vec<httparse::Header<'a>>,
    body: Vec<u8>,
}

impl<'a> Response<'a> {
    pub fn new() -> Response<'a> {
        Response {
            code: None,
            headers: Vec::with_capacity(INITIAL_HEADER_COUNT),
            body: Vec::with_capacity(INITIAL_BODY_SIZE),
        }
    }

    pub fn bad_request(msg: Option<&'a str>) -> Response<'a> {
        Response {
            code: Some(status::Code::BadRequest400),
            headers: vec![httparse::Header { name: "Content-Type", value: b"text/plain" }],
            body: msg.map_or(vec![], |m| m.to_string().into_bytes()),
        }
    }

    pub fn set_code(&mut self, code: status::Code) {
        self.code = Some(code);
    }

    pub fn add_header(&mut self, name: &'a str, value: &'a [u8]) {
        self.headers.push(httparse::Header { name: name, value: value});
    }

    pub fn add_body(&mut self, body: &'a [u8]) {
        self.body.push_all(body);
    }

    pub fn finalize(&mut self, buf: &mut Vec<u8>) {
        if self.code.is_none() {
            self.code = Some(status::Code::InternalServerError500);
        }

        let preamble = format!(
            "HTTP/1.1 {}\r\nDate: {}\r\nContent-Length: {}\r\n",
            self.code.as_ref().expect("all responses have a code"),
            time::strftime("%a, %d %b %y %T GMT", &time::now_utc()).unwrap(),
            self.body.len());

        buf.append(&mut preamble.into_bytes());

        for header in self.headers.iter() {
            buf.push_all(header.name.as_bytes());
            buf.push_all(b": ");
            buf.push_all(header.value);
            buf.push_all(b"\r\n");
        }
        buf.push_all(b"\r\n");

        buf.append(&mut self.body);
    }
}
