use proc_macro2::{TokenStream, Span};
use quote::format_ident;
use syn::{Result, Error, parse2, ItemStruct, Attribute, PathSegment};


pub(crate) fn parse_struct(macro_name: &str, input: TokenStream) -> Result<ItemStruct> {
    let mut struct_tokens = parse2::<ItemStruct>(input)?;

    if struct_tokens.semi_token.is_some() {
        return Err(Error::new(Span::call_site(), format!(
            "`#[{macro_name}]` doesn't support tuple or tag struct"
        )))
    }

    if struct_tokens.generics.type_params().count() > 0 {
        return Err(Error::new(Span::call_site(), format!(
            "`#[{macro_name}]` doesn't support type params"
        )))
    }

    if struct_tokens.generics.const_params().count() > 0 {
        return Err(Error::new(Span::call_site(), format!(
            "`#[{macro_name}]` doesn't support const params"
        )))
    }

    struct_tokens.attrs = struct_tokens.attrs.into_iter()
        .filter(|attr| is_not(attr, macro_name))
        .collect();

    Ok(struct_tokens)
}


fn is_not(attr: &Attribute, name: &str) -> bool {
    let mut segments = attr.path.segments.iter().peekable();

    let is_just_ident = |s: &PathSegment, ident: &str| {
        s.arguments.is_empty() &&
        s.ident == format_ident!("{ident}")
    };

    let is_ident = |s: &PathSegment, ident: &str| {
        s.ident == format_ident!("{ident}")
    };

    match segments.next_if(|s| is_ident(s, name)) {
        Some(_) => false, /* it's me:
            #[Queries] | #[Payload(JSON | Form)]
        */
        None    => {
            match segments.next_if(|s| is_just_ident(s, "ohkami")) {
                None    => true,
                Some(_) => match segments.next_if(|s| is_just_ident(s, name)) {
                    None    => true,
                    Some(_) => false, /* it's me:
                        #[ohkami::Queries]
                        | #[::ohkami::Queries]
                        | #[ohkami::Payload(JSON | Form)]
                        | #[::ohkami::Payload(JSON | Form)]
                    */
                }
            }
        }
    }
}
