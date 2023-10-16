mod parse_payload; pub use parse_payload::*;
mod from_request;  pub use from_request::*;
#[cfg(test)] mod _parse_test;

use std::{borrow::Cow};
use byte_reader::{Reader};
use percent_encoding::{percent_decode};
use crate::{
    __dep__::{TcpStream, AsyncReader},
    layer0_lib::{List, Method, ContentType, Slice}
};

pub(crate) const METADATA_SIZE: usize = 1024;
pub(crate) const PAYLOAD_LIMIT: usize = 65536;

pub(crate) const QUERIES_LIMIT: usize = 4;
pub(crate) const HEADERS_LIMIT: usize = 32;


pub struct Request {
    _metadata: [u8; METADATA_SIZE],
    payload:   Option<(ContentType, Vec<u8>)>,
    method:  Method,
    path:    Slice,
    queries: List<(Slice, Slice), QUERIES_LIMIT>,
    headers: List<(Slice, Slice), HEADERS_LIMIT>,
} const _: () = {
    unsafe impl Send for Request {}
    unsafe impl Sync for Request {}
};

impl Request {
    pub(crate) async fn new(stream: &mut TcpStream) -> Self {
        let mut _metadata = [b'0'; METADATA_SIZE];
        stream.read(&mut _metadata).await.unwrap();

        let mut r = Reader::new(&_metadata);

        let method = Method::from_bytes(r.read_while(|b| b != &b' '));
        r.consume(" ").unwrap();
        
        let path = unsafe {Slice::from_bytes(r.read_while(|b| b != &b'?' && b != &b' '))};

        let mut queries = List::<_, {QUERIES_LIMIT}>::new();
        if r.consume_oneof([" ", "?"]).unwrap() == 1 {
            while r.peek().is_some() {
                let key = unsafe {Slice::from_bytes(r.read_while(|b| b != &b'='))};
                r.consume("=").unwrap();
                let val = unsafe {Slice::from_bytes(r.read_while(|b| b != &b'&' && b != &b' '))};

                queries.append((key, val));
                if r.consume_oneof(["&", " "]).unwrap() == 1 {break}
            }
        }

        r.consume("HTTP/1.1\r\n").expect("Ohkami can only handle HTTP/1.1");

        let mut headers = List::<_, {HEADERS_LIMIT}>::new();
        let (mut content_type, mut content_length) = (None, 0usize);
        while r.consume("\r\n").is_none() {
            let _key = r.read_while(|b| b != &b':');
            let _content_flag = if _key.eq_ignore_ascii_case(b"Content-Type") {
                Some(true)
            } else if _key.eq_ignore_ascii_case(b"Content-Length") {
                Some(false)
            } else {None};
            let key = unsafe {Slice::from_bytes(_key)};

            r.consume(": ").unwrap();

            let _val = r.read_while(|b| b != &b'\r');
            match _content_flag {None => (),
                Some(true)  => (|| content_type   = ContentType::from_bytes(unsafe {_val}))(),
                Some(false) => (|| content_length = _val.into_iter().fold(0, |len, d| 10*len + *d as usize))(),
            }
            let val = unsafe {Slice::from_bytes(_val)};
            r.consume("\r\n").unwrap();

            headers.append((key, val));
        }

        let payload = (content_length > 0).then_some((
            content_type.unwrap_or(ContentType::Text),
            Request::read_payload(stream, &_metadata, r.index, content_length.min(PAYLOAD_LIMIT)).await
        ));

        Self { _metadata, payload, method, path, queries, headers }
    }

    async fn read_payload(
        stream:    &mut TcpStream,
        ref_metadata: &[u8],
        starts_at:    usize,
        size:         usize,
    ) -> Vec<u8> {
        assert!(starts_at <= METADATA_SIZE);

        let mut bytes = vec![0; size];
        bytes[..(METADATA_SIZE - starts_at)]
            .copy_from_slice(&ref_metadata[starts_at..]);
        stream.read_exact(bytes[(METADATA_SIZE - starts_at)..]
            .as_mut()).await.unwrap();
        bytes
    }
}

impl Request {
    #[inline] pub fn method(&self) -> Method {
        self.method
    }
    #[inline] pub fn path(&self) -> &str {
        unsafe {std::mem::transmute(
            &*(percent_decode(self.path.into_bytes()).decode_utf8_lossy())
        )}
    }
    #[inline] pub fn query<Value: FromBuffer>(&self, key: &str) -> Option<Result<Value, Cow<'static, str>>> {
        for (key_, value) in self.queries.iter() {
            if key.eq_ignore_ascii_case(&percent_decode(unsafe {key_.into_bytes()}).decode_utf8_lossy()) {
                return Some(Value::parse((&percent_decode(unsafe {value.into_bytes()}).decode_utf8_lossy()).as_bytes()))
            }
        }
        None
    }
    #[inline] pub fn header(&self, key: &str) -> Option<&str> {
        for (key_, value) in self.headers.iter() {
            if key.as_bytes().eq_ignore_ascii_case(unsafe {key_.into_bytes()}) {
                return Some(unsafe {std::str::from_utf8_unchecked(value.into_bytes())})
            }
        }
        None
    }
    #[inline] pub fn payload(&self) -> Option<(&ContentType, &[u8])> {
        let (content_type, body) = (&self.payload).as_ref()?;
        Some((content_type, &body))
    }
}

impl Request {
    #[inline(always)] pub(crate) fn path_bytes(&self) -> &[u8] {
        unsafe {self.path.into_bytes()}
    }
}

const _: () = {
    impl std::fmt::Debug for Request {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let queires = {
                let List { list, next } = &self.queries;
                list[..*next].into_iter()
                    .map(|cell| {
                        let (k, v) = unsafe {cell.assume_init_ref()};
                        format!("{} = {}",
                            percent_decode(unsafe {k.into_bytes()}).decode_utf8_lossy(),
                            percent_decode(unsafe {v.into_bytes()}).decode_utf8_lossy(),
                        )
                    })
            }.collect::<Vec<_>>();

            let headers = {
                let List { list, next } = &self.headers;
                list[..*next].into_iter()
                    .map(|cell| unsafe {
                        let (k, v) = cell.assume_init_ref();
                        format!("{}: {}",
                            std::str::from_utf8_unchecked(k.into_bytes()),
                            std::str::from_utf8_unchecked(v.into_bytes()),
                        )
                    })
            }.collect::<Vec<_>>();

            if let Some((_, payload)) = self.payload() {
                f.debug_struct("Request")
                    .field("method",  &self.method)
                    .field("path",    &self.path())
                    .field("queries", &queires)
                    .field("headers", &headers)
                    .field("payload", &String::from_utf8_lossy(payload))
                    .finish()

            } else {
                f.debug_struct("Request")
                    .field("method",  &self.method)
                    .field("path",    &self.path())
                    .field("queries", &queires)
                    .field("headers", &headers)
                    .finish()
            }
        }
    }
};




#[cfg(test)]
struct DebugRequest {
    method: Method,
    path: &'static str,
    queries: &'static [(&'static str, &'static str)],
    headers: &'static [(&'static str, &'static str)],
    payload: Option<(ContentType, &'static str)>,
}
#[cfg(test)]
const _: () = {
    impl DebugRequest {
        pub(crate) fn assert_parsed_from(self, req_str: &'static str) {
            let DebugRequest { method, path, queries, headers, payload } = self;
            let req = parse::parse(Buffer::from_raw_str(req_str));

            assert_eq!(req.method(), method);
            assert_eq!(req.path(), path);
            assert_eq!(req.payload().map(|(ct, s)| (ct.clone(), std::str::from_utf8(s).unwrap())), payload);
            for (k, v) in queries {
                assert_eq!(req.query::<String>(k), Some(Ok((*v).to_owned())))
            }
            for (k, v) in headers {
                assert_eq!(req.header(k), Some(*v))
            }
        }
    }
};
