#![cfg(any(feature="rt_tokio",feature="rt_async-std"))]

use crate::header::{append, SameSitePolicy, SetCookie};
use super::ResponseHeaders;
use ohkami_lib::time::UTCDateTime;


#[test] fn insert_and_write() {
    let __now__ = UTCDateTime::from_duration_since_unix_epoch(
        std::time::Duration::from_secs(crate::utils::unix_timestamp())
    ).into_imf_fixdate();

    let mut h = ResponseHeaders::new();
    h.set().Server("A");
    {
        let mut buf = Vec::new();
        h._write_to(&mut buf);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("\
            Date: {__now__}\r\n\
            Server: A\r\n\
            \r\n\
        "));
    }

    let mut h = ResponseHeaders::new();
    h.set().Server("A").ContentType("application/json");
    h.set().Server("B");
    h.set().ContentType("text/html");
    {
        let mut buf = Vec::new();
        h._write_to(&mut buf);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("\
            Date: {__now__}\r\n\
            Server: B\r\n\
            Content-Type: text/html\r\n\
            \r\n\
        "));
    }
}

#[test] fn append_header() {
    let __now__ = UTCDateTime::from_duration_since_unix_epoch(
        std::time::Duration::from_secs(crate::utils::unix_timestamp())
    ).into_imf_fixdate();

    let mut h = ResponseHeaders::new();

    h.set().Server(append("X"));
    assert_eq!(h.Server(), Some("X"));
    {
        let mut buf = Vec::new();
        h._write_to(&mut buf);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("\
            Date: {__now__}\r\n\
            Server: X\r\n\
            \r\n\
        "));
    }

    h.set().Server(append("Y"));
    assert_eq!(h.Server(), Some("X, Y"));
    {
        let mut buf = Vec::new();
        h._write_to(&mut buf);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("\
            Date: {__now__}\r\n\
            Server: X, Y\r\n\
            \r\n\
        "));
    }
}

#[test] fn append_custom_header() {
    let __now__ = UTCDateTime::from_duration_since_unix_epoch(
        std::time::Duration::from_secs(crate::utils::unix_timestamp())
    ).into_imf_fixdate();

    let mut h = ResponseHeaders::new();

    h.set().custom("Custom-Header", append("A"));
    assert_eq!(h.custom("Custom-Header"), Some("A"));
    {
        let mut buf = Vec::new();
        h._write_to(&mut buf);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("\
            Date: {__now__}\r\n\
            Custom-Header: A\r\n\
            \r\n\
        "));
    }

    h.set().custom("Custom-Header", append("B"));
    assert_eq!(h.custom("Custom-Header"), Some("A, B"));
    {
        let mut buf = Vec::new();
        h._write_to(&mut buf);
        assert_eq!(std::str::from_utf8(&buf).unwrap(), format!("\
            Date: {__now__}\r\n\
            Custom-Header: A, B\r\n\
            \r\n\
        "));
    }
}

#[test] fn parse_setcookie_headers() {
    let mut h = ResponseHeaders::new();
    h.set().SetCookie("id", "42", |d|d.Path("/").SameSiteLax().Secure());
    assert_eq!(h.SetCookie().collect::<Vec<_>>(), [
        SetCookie {
            Cookie:   ("id", "42".into()),
            Expires:  None,
            MaxAge:   None,
            Domain:   None,
            Path:     Some("/".into()),
            Secure:   Some(true),
            HttpOnly: None,
            SameSite: Some(SameSitePolicy::Lax),
        }
    ]);

    let mut h = ResponseHeaders::new();
    h.set()
        .SetCookie("id", "10", |d|d.Path("/").SameSiteLax().Secure())
        .SetCookie("id", "42", |d|d.MaxAge(1280).HttpOnly().Path("/where").SameSiteLax().Secure());
    assert_eq!(h.SetCookie().collect::<Vec<_>>(), [
        SetCookie {
            Cookie:   ("id", "10".into()),
            Expires:  None,
            MaxAge:   None,
            Domain:   None,
            Path:     Some("/".into()),
            Secure:   Some(true),
            HttpOnly: None,
            SameSite: Some(SameSitePolicy::Lax),
        },
        SetCookie {
            Cookie:   ("id", "42".into()),
            Expires:  None,
            MaxAge:   Some(1280),
            Domain:   None,
            Path:     Some("/where".into()),
            Secure:   Some(true),
            HttpOnly: Some(true),
            SameSite: Some(SameSitePolicy::Lax),
        },
    ]);
}
