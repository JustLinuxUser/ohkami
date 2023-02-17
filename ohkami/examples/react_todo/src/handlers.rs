pub(crate) mod user;
pub(crate) mod todo;

use ohkami::{
    error::Result,
    context::Context,
    response::Response,
};

pub(crate) async fn root(c: Context) -> Result<Response> {
    c.OK("Hello, World!")
}