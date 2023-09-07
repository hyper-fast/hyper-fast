use std::iter::repeat;

use http::HeaderValue;
use libc::{c_char, c_int, size_t};

lazy_static! {
    pub static ref HOSTNAME: String = hostname();
    pub static ref HOSTNAME_HEADER: HeaderValue = hostname_header();
}

pub const BR_CONTENT_ENCODING: &[u8] = b"br";
pub const DEFLATE_CONTENT_ENCODING: &[u8] = b"deflate";
pub const GZIP_CONTENT_ENCODING: &[u8] = b"gzip";

pub fn hostname_header() -> HeaderValue {
    HeaderValue::from_static(get_hostname())
}

pub fn get_hostname() -> &'static str {
    &HOSTNAME
}

pub fn get_hostname_header() -> &'static HeaderValue {
    &HOSTNAME_HEADER
}

extern "C" {
    pub fn gethostname(name: *mut c_char, size: size_t) -> c_int;
}

/// Calls `gethostname`
pub fn hostname() -> String {
    // Create a buffer for the hostname to be copied into
    let buffer_len: usize = 255;
    let mut buffer: Vec<u8> = repeat(0).take(buffer_len).collect();

    let error = unsafe { gethostname(buffer.as_mut_ptr() as *mut c_char, buffer_len as size_t) };

    if error != 0 {
        panic!("get hostname failed");
    }

    // Find the end of the string and truncate the vector to that length
    let len = buffer.iter().position(|b| *b == 0).unwrap_or(buffer_len);
    buffer.truncate(len);

    // Create an owned string from the buffer, transforming UTF-8 errors into IO errors
    match String::from_utf8(buffer) {
        Ok(hostname) => return hostname,
        Err(err) => {
            let err_msg = format!("Failed to convert to String {}", err);
            panic!("{}", err_msg);
        }
    }
}
