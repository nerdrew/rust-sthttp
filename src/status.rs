use std;

#[derive(Debug)]
pub enum Code {
    Ok200,
    Created201,
    BadRequest400,
    InternalServerError500,
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Code::Ok200 => write!(f, "200 OK"),
            Code::Created201 => write!(f, "201 Created"),
            Code::BadRequest400 => write!(f, "400 Bad Request"),
            Code::InternalServerError500 => write!(f, "500 Internal Server Error"),
        }
    }
}
