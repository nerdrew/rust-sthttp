use httparse;

pub struct Request<'a> {
    pub method: Option<&'a str>,
    pub path: Option<&'a str>,
    pub headers: &'a [httparse::Header<'a>],
    pub body: &'a [u8],
}

impl<'a> Request<'a> {
    pub fn new(req: &'a httparse::Request, body: &'a [u8]) -> Request<'a> {
        Request {
            method: req.method,
            path: req.path,
            headers: req.headers,
            body: body,
        }
    }
}
