extern crate sthttp;

struct CustomHandler;

impl sthttp::server::Handler for CustomHandler {
    fn handle(&self, req: sthttp::request::Request, res: &mut sthttp::response::Response) {
        match req.path {
            Some(ref path) if path.as_bytes() == b"/echo" => {
                res.set_code(sthttp::status::Code::Ok200);
                res.add_header("Content-Type", b"text/plain");
                res.add_body(&req.body[..]);
            },
            Some(_) => {
                res.set_code(sthttp::status::Code::Ok200);
                res.add_header("Content-Type", b"text/plain");
            },
            None => {}
        }
    }
}

#[test]
fn it_slices_and_dices() {
    std::thread::spawn(move ||{
        let handler = CustomHandler;
        sthttp::server::start("0.0.0.0:6567".parse().unwrap(), &handler);
    });

    let client = sthttp::client::HttpClient::new();
}
