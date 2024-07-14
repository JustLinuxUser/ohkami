use crate::header::{IndexMap, Append, SetCookie, SetCookieBuilder};
use std::borrow::Cow;
use ohkami_lib::{num, time::UTCDateTime};
use rustc_hash::FxHashMap;


#[derive(Clone)]
pub struct Headers {
    standard: IndexMap<N_SERVER_HEADERS, Cow<'static, str>>,
    custom:   Option<Box<FxHashMap<&'static str, Cow<'static, str>>>>,
    special:  Special,
    pub(crate) size: usize,
}

#[derive(Clone)]
#[allow(non_snake_case)]
struct Special {
    ContentLength: Option<usize>,
    Date:          Option<UTCDateTime>,
    SetCookie:     Option<Box<Vec<Cow<'static, str>>>>,
} impl Special {
    #[inline]
    const fn new() -> Self {
        Self {
            ContentLength: None,
            Date:          None,
            SetCookie:     None
        }
    }

    #[cfg(any(
        feature="rt_tokio",feature="rt_async-std",
        feature="DEBUG"
    ))]
    /// SAFETY:
    /// 1. `buf` has enough capacity of remaining
    /// 2. `.Date` is initialized
    #[inline]
    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        if let Some(d) = self.Date {
            crate::push_unchecked!(buf <- "Date: ");
            d.fmt_imf_fixdate_unchecked(buf);
            crate::push_unchecked!(buf <- "\r\n");
        }

        if let Some(n) = self.ContentLength {
            crate::push_unchecked!(buf <- "Content-Length: ");
            num::encode_itoa_unchecked(n, buf);
            crate::push_unchecked!(buf <- "\r\n");
        }

        if let Some(setcookies) = &self.SetCookie {
            for setcookie in &**setcookies {
                crate::push_unchecked!(buf <- "Set-Cookie: ");
                crate::push_unchecked!(buf <- setcookie);
                crate::push_unchecked!(buf <- "\r\n");
            }
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&str, Cow<'_, str>)> {
        None.into_iter()
            .chain(self.Date.map(|v| (
                "Date", v.to_imf_fixdate().into()
            )))
            .chain(self.ContentLength.map(|v| (
                "Content-Length", num::itoa(v).into()
            )))
            .chain(self.SetCookie.as_deref().into_iter().flatten().map(|v| (
                "Set-Cookie", Cow::Borrowed(&**v)
            )))
    }
}

pub struct SetHeaders<'set>(
    &'set mut Headers
); impl Headers {
    #[inline] pub fn set(&mut self) -> SetHeaders<'_> {
        SetHeaders(self)
    }
}

pub trait HeaderAction<'action> {
    fn perform(self, set: SetHeaders<'action>, key: Header) -> SetHeaders<'action>;
} const _: () = {
    // remove
    impl<'a> HeaderAction<'a> for Option<()> {
        #[inline] fn perform(self, set: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set.0.remove(key);
            set
        }
    }

    // append
    impl<'a> HeaderAction<'a> for Append {
        #[inline] fn perform(self, set: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set.0.append(key, self.0.into());
            set
        }
    }

    // insert
    impl<'a> HeaderAction<'a> for &'static str {
        #[inline(always)] fn perform(self, set: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set.0.insert(key, self.into());
            set
        }
    }
    impl<'a> HeaderAction<'a> for String {
        #[inline(always)] fn perform(self, set: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set.0.insert(key, self.into());
            set
        }
    }
    impl<'a> HeaderAction<'a> for std::borrow::Cow<'static, str> {
        fn perform(self, set: SetHeaders<'a>, key: Header) -> SetHeaders<'a> {
            set.0.insert(key, self.into());
            set
        }
    }
};

pub trait CustomHeaderAction<'action> {
    fn perform(self, set: SetHeaders<'action>, key: &'static str) -> SetHeaders<'action>;
} const _: () = {
    /* remove */
    impl<'set> CustomHeaderAction<'set> for Option<()> {
        #[inline]
        fn perform(self, set: SetHeaders<'set>, key: &'static str) -> SetHeaders<'set> {
            set.0.remove_custom(key);
            set
        }
    }

    /* append */
    impl<'set> CustomHeaderAction<'set> for Append {
        fn perform(self, set: SetHeaders<'set>, key: &'static str) -> SetHeaders<'set> {
            set.0.append_custom(key, self.0.into());
            set
        }
    }

    /* insert */
    impl<'set> CustomHeaderAction<'set> for &'static str {
        #[inline(always)] fn perform(self, set: SetHeaders<'set>, key: &'static str) -> SetHeaders<'set> {
            set.0.insert_custom(key, self.into());
            set
        }
    }
    impl<'set> CustomHeaderAction<'set> for String {
        #[inline(always)] fn perform(self, set: SetHeaders<'set>, key: &'static str) -> SetHeaders<'set> {
            set.0.insert_custom(key, self.into());
            set
        }
    }
    impl<'set> CustomHeaderAction<'set> for Cow<'static, str> {
        fn perform(self, set: SetHeaders<'set>, key: &'static str) -> SetHeaders<'set> {
            set.0.insert_custom(key, self.into());
            set
        }
    }
};

macro_rules! Header {
    ($N:literal; $( $konst:ident: $name_bytes:literal, )*) => {
        pub(crate) const N_SERVER_HEADERS: usize = $N;
        const _: [Header; N_SERVER_HEADERS] = [$(Header::$konst),*];

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
            pub const fn as_str(&self) -> &'static str {
                unsafe {std::str::from_utf8_unchecked(self.as_bytes())}
            }
            #[inline(always)] const fn len(&self) -> usize {
                match self {
                    $(
                        Self::$konst => $name_bytes.len(),
                    )*
                }
            }

            // Mainly used in tests
            pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
                (0..N_SERVER_HEADERS)
                    .map(|i| unsafe {std::mem::transmute::<_, Header>(i as u8)})
                    .find(|h| h.as_bytes().eq_ignore_ascii_case(bytes))
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
                #[inline]
                pub fn $konst(self, action: impl HeaderAction<'set>) -> Self {
                    action.perform(self, Header::$konst)
                }
            )*

            #[inline]
            pub fn custom(self, name: &'static str, action: impl CustomHeaderAction<'set>) -> Self {
                action.perform(self, name)
            }
        }

        #[allow(non_snake_case)]
        impl Headers {
            $(
                #[inline]
                pub fn $konst(&self) -> Option<&str> {
                    self.get(Header::$konst)
                }
            )*

            #[inline]
            pub fn custom(&self, name: &'static str) -> Option<&str> {
                self.get_custom(name)
            }
        }
    };
} Header! {43; /* ContentLength, Date, SetCookie : specialized */
    AcceptRanges:                    b"Accept-Ranges",
    AccessControlAllowCredentials:   b"Access-Control-Allow-Credentials",
    AccessControlAllowHeaders:       b"Access-Control-Allow-Headers",
    AccessControlAllowMethods:       b"Access-Control-Allow-Methods",
    AccessControlAllowOrigin:        b"Access-Control-Allow-Origin",
    AccessControlExposeHeaders:      b"Access-Control-Expose-Headers",
    AccessControlMaxAge:             b"Access-Control-Max-Age",
    Age:                             b"Age",
    Allow:                           b"Allow",
    AltSvc:                          b"Alt-Svc",
    CacheControl:                    b"Cache-Control",
    CacheStatus:                     b"Cache-Status",
    CDNCacheControl:                 b"CDN-Cache-Control",
    Connection:                      b"Connection",
    ContentDisposition:              b"Content-Disposition",
    ContentEncoding:                 b"Content-Ecoding",
    ContentLanguage:                 b"Content-Language",
    /* ContentLength:                   b"Content-Length", */
    ContentLocation:                 b"Content-Location",
    ContentRange:                    b"Content-Range",
    ContentSecurityPolicy:           b"Content-Security-Policy",
    ContentSecurityPolicyReportOnly: b"Content-Security-Policy-Report-Only",
    ContentType:                     b"Content-Type",
    /* Date:                            b"Date", */
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
    /* SetCookie:                       b"Set-Cookie" */
    StrictTransportSecurity:         b"Strict-Transport-Security",
    Trailer:                         b"Trailer",
    TransferEncoding:                b"Transfer-Encoding",
    Upgrade:                         b"Upgrade",
    Vary:                            b"Vary",
    Via:                             b"Via",
    XContentTypeOptions:             b"X-Content-Type-Options",
    XFrameOptions:                   b"X-Frame-Options",
    WWWAuthenticate:                 b"WWW-Authenticate",
}

const _: () = {
    #[allow(non_snake_case)]
    impl Headers {
        pub fn SetCookie(&self) -> impl Iterator<Item = SetCookie<'_>> {
            self.special.SetCookie.as_ref().map(|setcookies|
                setcookies.iter().filter_map(|raw| match SetCookie::from_raw(raw) {
                    Ok(valid) => Some(valid),
                    Err(_err) => {
                        #[cfg(debug_assertions)] crate::warning!(
                            "Invalid `Set-Cookie`: {_err}"
                        );
                        None
                    }
                })
            ).into_iter().flatten()
        }

        #[allow(unused)]
        #[inline]
        pub(crate) const fn ContentLength(&self) -> Option<usize> {
            self.special.ContentLength
        }

        #[allow(unused)]
        pub(crate) const fn Date(&self) -> Option<UTCDateTime> {
            self.special.Date
        }
    }

    #[allow(non_snake_case)]
    impl<'s> SetHeaders<'s> {
        /// Add new `Set-Cookie` header in the response.
        /// 
        /// - When you call this N times, the response has N different
        ///   `Set-Cookie` headers.
        /// - Cookie value (second argument) is precent encoded when the
        ///   response is sended.
        /// 
        /// ---
        /// *example.rs*
        /// ```
        /// use ohkami::Response;
        /// 
        /// fn mutate_header(res: &mut Response) {
        ///     res.headers.set()
        ///         .Server("ohkami")
        ///         .SetCookie("id", "42", |d|d.Path("/").SameSiteLax())
        ///         .SetCookie("name", "John", |d|d.Path("/where").SameSiteStrict());
        /// }
        /// ```
        #[inline]
        pub fn SetCookie(self,
            name:  &'static str,
            value: impl Into<Cow<'static, str>>,
            directives: impl FnOnce(SetCookieBuilder)->SetCookieBuilder
        ) -> Self {
            let setcookie: Cow<'static, str> = directives(SetCookieBuilder::new(name, value)).build().into();
            self.0.size += "Set-Cookie: ".len() + setcookie.len() + "\r\n".len();
            match self.0.special.SetCookie.as_mut() {
                None             => self.0.special.SetCookie = Some(Box::new(vec![setcookie])),
                Some(setcookies) => setcookies.push(setcookie),
            }
            self
        }

        #[inline]
        pub(crate) fn ContentLength(self, length: usize) -> Self {
            self.0.special.ContentLength = Some(length);
            self.0.size += "Content-Length: ".len() + if length < 1<<9 {3} else {const {1 + usize::MAX.ilog10() as usize}} + "\r\n".len();
            self
        }
        #[inline]
        pub(crate) fn noContentLength(self) -> Self {
            if self.0.special.ContentLength.take().is_some() {
                self.0.size -= "Content-Length: ".len() + /*at least*/1/*digit*/ + "\r\n".len();
            }
            self
        }

        #[inline]
        pub(crate) fn Date(self, date: UTCDateTime) -> Self {
            self.0.special.Date = Some(date);
            self.0.size += "Date: ".len() + UTCDateTime::IMF_FIXDATE_LEN + "\r\n".len();
            self
        }
    }
};

impl Headers {
    #[inline(always)]
    pub(crate) fn insert(&mut self, name: Header, value: Cow<'static, str>) {
        let (name_len, value_len) = (name.len(), value.len());
        match unsafe {self.standard.get_mut(name as usize)} {
            None => {
                self.size += name_len + ": ".len() + value_len + "\r\n".len();
                unsafe {self.standard.set(name as usize, value)}
            }
            Some(old) => {
                self.size -= old.len(); self.size += value_len;
                *old = value
            }
        }
    }
    #[inline]
    pub(crate) fn insert_custom(&mut self, name: &'static str, value: Cow<'static, str>) {
        let self_len = value.len();
        match &mut self.custom {
            None => {
                self.custom = Some(Box::new(FxHashMap::from_iter([(name, value)])));
                self.size += name.len() + ": ".len() + self_len + "\r\n".len()
            }
            Some(custom) => {
                if let Some(old) = custom.insert(name, value) {
                    self.size -= old.len(); self.size += self_len;
                } else {
                    self.size += name.len() + ": ".len() + self_len + "\r\n".len()
                }
            }
        }
    }

    #[inline]
    pub(crate) fn remove(&mut self, name: Header) {
        let name_len = name.len();
        if let Some(v) = unsafe {self.standard.get(name as usize)} {
            self.size -= name_len + ": ".len() + v.len() + "\r\n".len()
        }
        unsafe {self.standard.delete(name as usize)}
    }
    pub(crate) fn remove_custom(&mut self, name: &'static str) {
        if let Some(c) = self.custom.as_mut() {
            if let Some(v) = c.remove(name) {
                self.size -= name.len() + ": ".len() + v.len() + "\r\n".len()
            }
        }
    }

    #[inline(always)]
    pub(crate) fn get(&self, name: Header) -> Option<&str> {
        match unsafe {self.standard.get(name as usize)} {
            Some(v) => Some(&*v),
            None => None
        }
    }
    #[inline]
    pub(crate) fn get_custom(&self, name: &str) -> Option<&str> {
        match self.custom.as_ref()?.get(name) {
            Some(v) => Some(&*v),
            None => None
        }
    }

    pub(crate) fn append(&mut self, name: Header, value: Cow<'static, str>) {
        let value_len = value.len();
        let target = unsafe {self.standard.get_mut(name as usize)};

        self.size += match target {
            Some(v) => {
                let mut buf = std::mem::take(v).into_owned();
                buf.push_str(", ");
                buf.push_str(&*value);
                let _ = std::mem::replace(v, Cow::Owned(buf));
                ", ".len() + value_len
            }
            None => {
                unsafe {self.standard.set(name as usize, value)}
                name.len() + ": ".len() + value_len + "\r\n".len()
            }
        };
    }
    pub(crate) fn append_custom(&mut self, name: &'static str, value: Cow<'static, str>) {
        let value_len = value.len();

        let custom = {
            if self.custom.is_none() {
                self.custom = Some(Box::new(FxHashMap::default()));
            }
            unsafe {self.custom.as_mut().unwrap_unchecked()}
        };

        self.size += match custom.get_mut(name) {
            Some(v) => {
                let mut buf = std::mem::take(v).into_owned();
                buf.push_str(", ");
                buf.push_str(&*value);
                let _ = std::mem::replace(v, Cow::Owned(buf));
                ", ".len() + value_len
            }
            None => {
                custom.insert(name, value);
                name.len() + ": ".len() + value_len + "\r\n".len()
            }
        };
    }
}

impl Headers {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            standard:  IndexMap::new(),
            custom:    None,
            special:   Special::new(),
            size:      "\r\n".len(),
        }
    }
    #[cfg(feature="DEBUG")]
    #[doc(hidden)]
    pub fn _new() -> Self {Self::new()}

    pub(crate) fn iter<'i>(&'i self) -> impl Iterator<Item = (&'i str, Cow<'i, str>)> {
        self.special.iter()
            .chain(self.standard.iter().map(|(i, v)| (
                unsafe {std::mem::transmute::<_, Header>(*i as u8)}.as_str(),
                Cow::Borrowed(&**v)
            )))
            .chain(self.custom.as_ref().into_iter()
                .flat_map(|hm| hm.iter().map(|(k, v)| (*k, Cow::Borrowed(&**v))))
            )
    }

    #[cfg(any(
        feature="rt_tokio",feature="rt_async-std",
        feature="DEBUG"
    ))]
    /// SAFETY: `buf` has remaining capacity of at least `self.size`
    pub(crate) unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        self.special.write_unchecked(buf);
        for (i, v) in self.standard.iter() {
            let h = std::mem::transmute::<_, Header>(*i as u8); {
                crate::push_unchecked!(buf <- h.as_bytes());
                crate::push_unchecked!(buf <- b": ");
                crate::push_unchecked!(buf <- v);
                crate::push_unchecked!(buf <- b"\r\n");
            }
        }
        if let Some(custom) = self.custom.as_ref() {
            for (k, v) in &**custom {
                crate::push_unchecked!(buf <- k.as_bytes());
                crate::push_unchecked!(buf <- b": ");
                crate::push_unchecked!(buf <- v);
                crate::push_unchecked!(buf <- b"\r\n");
            }
        }
        crate::push_unchecked!(buf <- b"\r\n");
    }

    #[cfg(feature="DEBUG")]
    pub fn _write_to(&self, buf: &mut Vec<u8>) {
        buf.reserve(self.size);
        unsafe {self.write_unchecked(buf)}
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
                if Some(&*v) != match Header::from_bytes(k.as_bytes()) {
                    Some(s) => other.get(s),
                    None    => other.get_custom(k)
                } {
                    return false
                }
            }
            true
        }
    }

    impl Headers {
        pub fn from_iter(iter: impl IntoIterator<Item = (
            &'static str,
            impl Into<Cow<'static, str>>)>
        ) -> Self {
            let mut this = Headers::new();
            for (k, v) in iter {
                match Header::from_bytes(k.as_bytes()) {
                    Some(h) => this.insert(h, v.into().into()),
                    None    => {this.set().custom(k, v.into());}
                }
            }
            this
        }
    }
};

#[cfg(feature="rt_worker")]
const _: () = {
    impl Into<::worker::Headers> for Headers {
        #[inline(always)]
        fn into(self) -> ::worker::Headers {
            let mut h = ::worker::Headers::new();
            for (k, v) in self.iter() {
                if let Err(_e) = h.append(k, &*v) {
                    #[cfg(feature="DEBUG")] println!("`worker::Headers::append` failed: {_e:?}");
                }
            }
            h
        }
    }
};
