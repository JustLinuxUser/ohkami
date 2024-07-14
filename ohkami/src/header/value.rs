use ohkami_lib::{num, Slice, time::UTCDateTime};


#[derive(Clone)]
pub enum Value {
    String(String),
    Slice(Slice),
    Time(UTCDateTime),
    Int(usize),
}

impl Value {
    #[inline(always)]
    pub fn size(&self) -> usize {
        match self {
            Self::String(s) => s.len(),
            Self::Slice(s) => s.len(),
            Self::Time(_) => UTCDateTime::IMF_FIXDATE_LEN,
            Self::Int(i) => if *i < 1<<9 {3} else {const {1 + usize::MAX.ilog10() as usize}}
        }
    }

    /// SAFETY: `buf` has at least `self.size()` remaining capacity
    #[inline(always)]
    pub unsafe fn push_unchecked(&self, buf: &mut Vec<u8>) {
        match self {
            Self::String(s) => crate::push_unchecked!(buf <- s),
            Self::Slice(s) => crate::push_unchecked!(buf <- s),
            Self::Time(t) => t.fmt_imf_fixdate_unchecked(buf),
            Self::Int(i) => num::encode_itoa_unchecked(*i, buf),
        }
    }

    /// SAFETY: `Slice` variant has, if exists, UTF-8 bytes
    #[inline(always)]
    pub unsafe fn as_str_unchecked(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(&*s),
            Self::Slice(s) => Some(std::str::from_utf8_unchecked(&*s)),
            _ => None
        }
    }

    /// SAFETY: any `Slice` variants have, if exists, UTF-8 bytes
    #[inline]
    pub unsafe fn append_unchecked(&mut self, another: Value) {
        let mut buf = match self {
            Self::String(s) => return {
                s.reserve(another.size());
                unsafe {another.push_unchecked(s.as_mut_vec())}
            },
            Self::Slice(s) => String::from_utf8_unchecked(Vec::from(&**s)),
            Self::Time(t) => t.to_imf_fixdate(),
            Self::Int(i) => num::itoa(*i),
        };
        buf.reserve(another.size());
        another.push_unchecked(buf.as_mut_vec());
        *self = Self::String(buf)
    }

    #[inline]
    pub fn stringify(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::String(s) => s.into(),
            Self::Slice(s) => std::str::from_utf8(&s).expect("Non UTF-8 header value").into(),
            Self::Time(t) => t.to_imf_fixdate().into(),
            Self::Int(i) => num::itoa(*i).into(),
        }
    }
}

impl From<std::borrow::Cow<'static, str>> for Value {
    #[inline(always)]
    fn from(cow: std::borrow::Cow<'static, str>) -> Self {
        match cow {
            std::borrow::Cow::Borrowed(b) => Self::Slice(b.into()),
            std::borrow::Cow::Owned(s)    => Self::String(s),
        }
    }
}
impl From<&'static str> for Value {
    #[inline(always)]
    fn from(s: &'static str) -> Self {
        Self::Slice(s.into())
    }
}
impl From<String> for Value {
    #[inline(always)]
    fn from(s: String) -> Self {
        Self::String(s)
    }
}
