use std::borrow::Cow;


pub struct Headers {
    values: [Value; N_SERVER_HEADERS],

    /// Size of whole the byte stream when this is written into HTTP response.
    size: usize,
}
#[derive(Clone)]
pub struct Value(
    Option<Cow<'static, str>>,
); impl Value {
    fn size(&self) -> usize {
        match &self.0 {
            None    => 0,
            Some(v) => v.len(),
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            Some(cows) => &cows,
            None       => "",
        }
    }
    pub fn append(&mut self, value: impl Into<Cow<'static, str>>) {
        match &mut self.0 {
            None    => self.0 = Some(value.into()),
            Some(v) => {
                let mut new = v.to_string();
                new.push_str(&value.into());
                *v = Cow::Owned(new);
            }
        }
    }
    pub fn replace(&mut self, new_value: impl Into<Cow<'static, str>>) {
        self.0 = Some(new_value.into())
    }
}

pub struct SetHeaders<'set>(
    &'set mut Headers
); impl Headers {
    #[inline(always)] pub(crate) fn set(&mut self) -> SetHeaders<'_> {
        SetHeaders(self)
    }
}
pub trait HeaderAction<'action> {
    fn perform(self, set_headers: SetHeaders<'action>, key: Header) -> SetHeaders<'action>;
} const _: () = {
    // remove
    impl<'a> HeaderAction<'a> for Option<()> {
        #[inline] fn perform(self, set_headers: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set_headers.0.remove(key);
            set_headers
        }
    }

    // insert
    impl<'a> HeaderAction<'a> for &'static str {
        #[inline] fn perform(self, set_headers: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set_headers.0.insert(key, Cow::Borrowed(self));
            set_headers
        }
    }
    impl<'a> HeaderAction<'a> for String {
        #[inline] fn perform(self, set_headers: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set_headers.0.insert(key, Cow::Owned(self));
            set_headers
        }
    }
    impl<'a> HeaderAction<'a> for std::borrow::Cow<'static, str> {
        fn perform(self, set_headers: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set_headers.0.insert(key, self);
            set_headers
        }
    }

    // append or something
    impl<'a, F: FnMut(&mut Value)> HeaderAction<'a> for F {
        #[inline] fn perform(mut self, set_headers: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            let before_size = set_headers.0.size;
            self(unsafe {set_headers.0.values.get_unchecked_mut(key as usize)});
            set_headers.0.size += set_headers.0.values[key as usize].size() - before_size;
            set_headers
        }
    }
};

macro_rules! Header {
    ($N:literal; $( $konst:ident: $name_bytes:literal, )*) => {
        pub(crate) const N_SERVER_HEADERS: usize = $N;
        pub(crate) const SERVER_HEADERS: [Header; N_SERVER_HEADERS] = [ $( Header::$konst ),* ];

        #[derive(Debug, PartialEq, Clone, Copy)]
        pub enum Header {
            $( $konst, )*
        }

        impl Header {
            #[inline] pub const fn as_bytes(&self) -> &'static [u8] {
                match self {
                    $(
                        Self::$konst => $name_bytes,
                    )*
                }
            }
            #[inline] pub const fn as_str(&self) -> &'static str {
                unsafe {std::str::from_utf8_unchecked(self.as_bytes())}
            }

            pub const fn from_bytes(bytes: &[u8]) -> Option<Self> {
                match bytes {
                    $(
                        $name_bytes => Some(Self::$konst),
                    )*
                    _ => None
                }
            }
        }

        impl<T: AsRef<[u8]>> PartialEq<T> for Header {
            fn eq(&self, other: &T) -> bool {
                self.as_bytes().eq_ignore_ascii_case(other.as_ref())
            }
        }

        #[allow(non_snake_case)]
        impl<'set> SetHeaders<'set> {
            $(
                pub fn $konst(self, action: impl HeaderAction<'set>) -> Self {
                    action.perform(self, Header::$konst)
                }
            )*
        }
        #[allow(non_snake_case)]
        impl Headers {
            $(
                pub fn $konst(&self) -> Option<&str> {
                    self.get(Header::$konst)
                }
            )*
        }
    };
} Header! {45;
    AcceptRanges:                    b"Accept-Ranges",
    AccessControlAllowCredentials:   b"Access-Control-Allow-Credentials",
    AccessControlAllowHeaders:       b"Access-Control-Allow-Headers",
    AccessControlAllowMethods:       b"Access-Control-Allow-Methods",
    AccessControlAllowOrigin:        b"Access-Control-Allow-Origin",
    AccessControlExposeHeaders:      b"Access-Control-Expose-Headers",
    AccessControlMaxAge:             b"Access-Control-Max-Age",
    Age:                             b"Age",
    Allow:                           b"Allow",
    AltSrv:                          b"Alt-Srv",
    CacheControl:                    b"Cache-Control",
    CacheStatus:                     b"Cache-Status",
    CDNCacheControl:                 b"CDN-Cache-Control",
    Connection:                      b"Connection",
    ContentDisposition:              b"Content-Disposition",
    ContentEncoding:                 b"Content-Ecoding",
    ContentLanguage:                 b"Content-Language",
    ContentLength:                   b"Content-Length",
    ContentLocation:                 b"Content-Location",
    ContentRange:                    b"Content-Range",
    ContentSecurityPolicy:           b"Content-Security-Policy",
    ContentSecurityPolicyReportOnly: b"Content-Security-Policy-Report-Only",
    ContentType:                     b"Content-Type",
    Date:                            b"Date",
    ETag:                            b"ETag",
    Expires:                         b"Expires",
    Link:                            b"Link",
    Location:                        b"Location",
    ProxyAuthenticate:               b"Proxy-Authenticate",
    ReferrerPolicy:                  b"Referrer-Policy",
    Refresh:                         b"Refresh",
    RetryAfter:                      b"Retry-After",
    SecWebSocketAccept:              b"Sec-WebSocket-Accept",
    SecWebSocketProtocol:            b"Sec-WebSocket-Protocol",
    SecWebSocketVersion:             b"Sec-WebSocket-Version",
    Server:                          b"Server",
    SetCookie:                       b"SetCookie",
    StrictTransportSecurity:         b"Strict-Transport-Security",
    Trailer:                         b"Trailer",
    TransferEncoding:                b"Transfer-Encoding",
    Upgrade:                         b"Upgrade",
    Vary:                            b"Vary",
    Via:                             b"Via",
    XContentTypeOptions:             b"X-Content-Type-Options",
    XFrameOptions:                   b"X-Frame-Options",
}

impl Headers {
    #[inline] pub(crate) fn insert(&mut self, name: Header, value: Cow<'static, str>) {
        let (name_len, value_len) = (name.as_bytes().len(), value.len());
        match unsafe {self.values.get_unchecked_mut(name as usize)}.0.replace(value) {
            None       => self.size += name_len + ": ".len() + value_len + "\r\n".len(),
            Some(prev) => {
                let prev_len = prev.len();
                if value_len > prev_len {
                    self.size += value_len - prev_len;
                } else {
                    self.size -= prev_len - value_len;
                }
            }
        }
    }

    #[inline] pub(crate) fn remove(&mut self, name: Header) {
        let name_len = name.as_bytes().len();
        let v = unsafe {self.values.get_unchecked_mut(name as usize)};
        if let Some(v) = v.0.take() {
            self.size -= name_len + ": ".len() + v.len() + "\r\n".len()
        }
    }

    pub(crate) fn get(&self, name: Header) -> Option<&str> {
        unsafe {self.values.get_unchecked(name as usize)}.0.as_ref().map(AsRef::as_ref)
    }
}
impl Headers {
    pub(crate) fn new() -> Self {
        Self {
            size:   "\r\n".len(),
            values: [
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
                Value(None), Value(None), Value(None), Value(None), Value(None),
            ]
        }
    }

    pub(crate) fn clone(&self) -> Self {
        Self { values: self.values.clone(), size: self.size }
    }

    pub(crate) const fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        struct Iter<'i> {
            map: &'i Headers,
            cur: usize,
        }
        impl<'i> Iterator for Iter<'i> {
            type Item = (&'i str, &'i str);
            fn next(&mut self) -> Option<Self::Item> {
                for i in self.cur..N_SERVER_HEADERS {
                    if let Value(Some(v)) = unsafe {self.map.values.get_unchecked(i)} {
                        self.cur = i + 1;
                        return Some((unsafe {SERVER_HEADERS.get_unchecked(i)}.as_str(), &v))
                    }
                }
                None
            }
        }

        Iter { map: self, cur: 0 }
    }

    pub(crate) fn write_to(self, buf: &mut Vec<u8>) {
        macro_rules! push {
            ($buf:ident <- $bytes:expr) => {
                unsafe {
                    let (buf_len, bytes_len) = ($buf.len(), $bytes.len());
                    std::ptr::copy_nonoverlapping(
                        $bytes.as_ptr(),
                        $buf.as_mut_ptr().add(buf_len),
                        bytes_len
                    );
                    $buf.set_len(buf_len + bytes_len);
                }
            };
        }

        buf.reserve(self.size);

        for h in &SERVER_HEADERS {
            if let Some(v) = &self.values[*h as usize].0 {
                push!(buf <- h.as_bytes());
                push!(buf <- b": ");
                push!(buf <- v);
                push!(buf <- b"\r\n");
            }
        }
        push!(buf <- b"\r\n");
    }
}

const _: () = {
    impl std::fmt::Debug for Headers {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_map()
                .entries(self.iter())
                .finish()
        }
    }

    impl PartialEq for Headers {
        fn eq(&self, other: &Self) -> bool {
            for (k, v) in self.iter() {
                if other.get(Header::from_bytes(k.as_bytes()).unwrap()) != Some(v) {
                    return false
                }
            }
            true
        }
    }

    impl Headers {
        pub fn from_iter(iter: impl IntoIterator<Item = (Header, impl Into<Cow<'static, str>>)>) -> Self {
            let mut this = Headers::new();
            for (k, v) in iter {
                this.insert(k, v.into())
            }
            this
        }
    }
};
