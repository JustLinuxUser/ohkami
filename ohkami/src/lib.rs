#![doc(html_root_url = "https://docs.rs/ohkami")]

#![cfg_attr(feature="nightly", feature(
    try_trait_v2,
))]


/*===== crate features =====*/

#[cfg(any(
    all(feature="rt_tokio", feature="rt_async-std")
))] compile_error!("
    Can't activate multiple `rt_*` feature!
");

#[cfg(not(any(
    feature="rt_tokio",
    feature="rt_async-std",
)))] compile_error!("
    Activate 1 of `rt_*` features：
    - rt_tokio
    - rt_async-std
");


/*===== runtime dependency injection layer =====*/

mod __rt__ {
    #[cfg(feature="rt_tokio")]
    pub(crate) use tokio::sync::Mutex;

    #[cfg(feature="rt_tokio")]
    pub(crate) use tokio::net::TcpListener;
    #[cfg(feature="rt_async-std")]
    pub(crate) use async_std::net::TcpListener;

    #[cfg(feature="rt_tokio")]
    pub(crate) use tokio::task;
    #[cfg(feature="rt_async-std")]
    pub(crate) use async_std::task;

    #[cfg(feature="rt_tokio")]
    pub(crate) use tokio::io::AsyncReadExt as AsyncReader;
    #[cfg(feature="rt_async-std")]
    pub(crate) use async_std::io::ReadExt as AsyncReader;

    #[cfg(feature="rt_tokio")]
    pub(crate) use tokio::io::AsyncWriteExt as AsyncWriter;
    #[cfg(feature="rt_async-std")]
    pub(crate) use async_std::io::WriteExt as AsyncWriter;

    #[cfg(feature="rt_async-std")]
    pub(crate) use async_std::stream::StreamExt;
}


/*===== modules =====*/

mod layer0_lib;
mod layer1_req_res;
mod layer2_context;
mod layer3_fang_handler;
mod layer4_router;
mod layer5_ohkami;
mod layer6_testing;


/*===== visibility managements =====*/

pub use layer0_lib         ::{Status, Method, ContentType};
pub use layer1_req_res     ::{Request, Response, FromRequest};
pub use layer2_context     ::{Context};
pub use layer3_fang_handler::{Route, Fang};
pub use layer5_ohkami      ::{Ohkami, IntoFang};

pub mod prelude {
    pub use crate::{Request, Response, Context, Route, Ohkami};
}

pub mod utils {
    pub use crate::layer1_req_res     ::{File};
    pub use crate::layer3_fang_handler::{builtin::*};
    pub use crate::layer6_testing     ::{Testing};
    pub use ohkami_macros             ::{Query, Payload};
}

#[doc(hidden)]
pub mod __internal__ {
    pub use crate::layer1_req_res::{
        parse_json,
        parse_formparts,
        parse_urlencoded,
        FromBuffer,
    };
}


/*===== usavility =====*/

#[cfg(test)] #[allow(unused)] async fn __() {
// fangs
    struct AppendHeader;
    impl IntoFang for AppendHeader {
        fn bite(self) -> Fang {
            Fang(|c: &mut Context, _: &mut Request| {
                c.headers.Server("ohkami");
            })
        }
    }

    struct Log;
    impl IntoFang for Log {
        fn bite(self) -> Fang {
            Fang(|res: Response| {
                println!("{res:?}");
                res
            })
        }
    }

// handlers
    async fn health_check(c: Context) -> Response {
        c.NoContent()
    }

    async fn hello(c: Context, name: String) -> Response {
        c.OK().text(format!("Hello, {name}!"))
    }

// run
    Ohkami::with((
        Log,
        AppendHeader,
        utils::cors("https://kanarusblog.software")
            .AllowCredentials()
            .AllowHeaders(["Content-Type"])
            .AllowMethods([Method::GET, Method::PUT, Method::POST, Method::DELETE])
            .MaxAge(3600)
    ), (
        "/hc".
            GET(health_check),
        "/hello/:name".
            GET(hello),
    )).howl(3000).await
}
