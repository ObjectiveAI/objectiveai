//! The issuer's two pages, as HTML.
//!
//! The sign-in page of the authorization endpoint and the device page
//! are the issuer's only HTML. Each is one document, made on the
//! server, with no script; every value put in it is escaped. Each is
//! sent never to be cached and never to be framed, so no other site
//! can show it inside its own.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

/// `text`, escaped for HTML: safe as an element's text and as a quoted
/// attribute's value.
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// A page: `title`, and `body`, HTML already escaped, with `status`.
pub fn page(status: StatusCode, title: &str, body: &str) -> Response {
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{title}</title><style>{STYLE}</style></head>\
         <body><main><h1>{title}</h1>{body}</main></body></html>",
        title = escape(title),
    );
    let headers = [
        (header::CONTENT_TYPE, "text/html; charset=utf-8"),
        (header::CACHE_CONTROL, "no-store"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'"),
    ];
    (status, headers, html).into_response()
}

/// A paragraph of `text`, escaped.
pub fn paragraph(text: &str) -> String {
    format!("<p>{}</p>", escape(text))
}

/// A hidden field `name` holding `value`, where there is a value.
pub fn hidden(name: &str, value: Option<&str>) -> String {
    match value {
        Some(value) => format!("<input type=\"hidden\" name=\"{}\" value=\"{}\">", escape(name), escape(value)),
        None => String::new(),
    }
}

/// The fields a person signs in with, and the two buttons: approve and
/// deny, the form's `action`.
pub fn sign_in(approve: &str) -> String {
    format!(
        "<label>Identity<input name=\"subject\" autocomplete=\"username\" required></label>\
         <label>Password<input name=\"password\" type=\"password\" autocomplete=\"current-password\" required></label>\
         <div><button name=\"action\" value=\"approve\">{}</button>\
         <button name=\"action\" value=\"deny\" formnovalidate>Deny</button></div>",
        escape(approve),
    )
}

/// The pages' one style.
const STYLE: &str = "body{font-family:system-ui,sans-serif;margin:0;padding:16px;background:#f6f6f4;color:#1c1c1a}\
main{max-width:360px;margin:48px auto}h1{font-size:1.4rem}label{display:block;margin:12px 0}\
input{display:block;width:100%;box-sizing:border-box;padding:8px;margin-top:4px;font:inherit}\
button{margin:12px 8px 0 0;padding:8px 16px;font:inherit}\
@media (prefers-color-scheme:dark){body{background:#1c1c1a;color:#f0f0ec}}";
