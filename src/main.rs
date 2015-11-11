#[macro_use] extern crate log;
extern crate bytes;
extern crate env_logger;
extern crate httparse;
extern crate mio;
extern crate sthttp;

struct CustomHandler;

impl sthttp::Handler for CustomHandler {
    fn handle(&self, req: sthttp::request::Request, res: &mut sthttp::response::Response) {
        debug!("request='{}'", unsafe { std::str::from_utf8_unchecked(req.body) });
        res.set_code(sthttp::status::Code::Ok200);
        res.add_header("Content-Type", b"text/plain");
        res.add_body(b"boom");
    }
}

fn main() {
    env_logger::init().unwrap();
    let handler = CustomHandler;
    sthttp::start("0.0.0.0:6567".parse().unwrap(), &handler);
}
