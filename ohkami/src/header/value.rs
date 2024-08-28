use ohkami_lib::{num, time::{UTCDateTime, IMF_FIXDATE_LEN}};


#[derive(Clone, PartialEq)]
pub(crate) enum HeaderValue {
    Slice(&'static str),
    String(String),
    Time(UTCDateTime),
    UInt(usize),
}

impl HeaderValue {
    /// SAFETY: `buf` has enough capacity for this HeaderValue
    #[inline]
    pub(crate) unsafe fn write_unchecked_to(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Slice(s)  => crate::push_unchecked!(buf <- s.as_bytes()),
            Self::String(s) => crate::push_unchecked!(buf <- s),
            Self::Time(t)   => t.fmt_imf_fixdate_unchecked(buf),
            Self::UInt(n)   => num::fmt_itoa_unchecked(*n, buf),
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Slice(s)  => s.len(),
            Self::String(s) => s.len(),
            Self::Time(_)   => IMF_FIXDATE_LEN,
            Self::UInt(n)   => if *n < (1<<9) {3} else {const {1 + usize::ilog10(usize::MAX) as usize}}
        }
    }

    pub(crate) fn append(&mut self, another: Self) {
        match self {
            Self::String(s) => unsafe {
                let buf = s.as_mut_vec();
                buf.set_len(buf.len() + another.len());
                another.write_unchecked_to(buf);
            }
            Self::Slice(s) => unsafe {
                let mut buf = Vec::from(s.as_bytes());
                buf.set_len(buf.len() + another.len());
                another.write_unchecked_to(&mut buf);
            }
            | Self::Time(_)
            | Self::UInt(_)
            => unimplemented!("appending to HeaderValue::{{Time, UInt}} is not supported")
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Slice(s)  => s,
            Self::String(s) => &*s,
            | Self::Time(_)
            | Self::UInt(_)
            => unimplemented!("`as_str` by HeaderValue::{{Time, UInt}} is not supported")
        }
    }
    pub(crate) fn as_cow_str(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Slice(s)  => std::borrow::Cow::Borrowed(s),
            Self::String(s) => std::borrow::Cow::Borrowed(&*s),
            Self::Time(t)   => std::borrow::Cow::Owned(t.clone().into_imf_fixdate()),
            Self::UInt(n)   => std::borrow::Cow::Owned(num::itoa(*n)),
        }
    }
}

impl From<std::borrow::Cow<'static, str>> for HeaderValue {
    fn from(cows: std::borrow::Cow<'static, str>) -> Self {
        match cows {
            std::borrow::Cow::Borrowed(s) => Self::Slice(s),
            std::borrow::Cow::Owned(s)    => Self::String(s)
        }
    }
}
impl From<&'static str> for HeaderValue {
    #[inline]
    fn from(s: &'static str) -> Self {
        Self::Slice(s)
    }
}
impl From<String> for HeaderValue {
    #[inline]
    fn from(s: String) -> Self {
        Self::String(s)
    }
}
impl From<usize> for HeaderValue {
    #[inline]
    fn from(n: usize) -> Self {
        Self::UInt(n)
    }
}
impl From<UTCDateTime> for HeaderValue {
    #[inline]
    fn from(t: UTCDateTime) -> Self {
        Self::Time(t)
    }
}
