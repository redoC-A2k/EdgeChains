use anyhow::{anyhow, Result};
use http::{request, HeaderName, HeaderValue};
use quickjs_wasm_rs::{JSContextRef, JSValueRef, Serializer};
use serde_bytes::ByteBuf;
use crate::{types::{HttpRequest, HttpResponse}, JSApiSet};
use javy::quickjs::{JSContextRef, JSValue, JSValueRef};
use quickjs_wasm_rs::Deserializer;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

mod outbound_http;


pub(super) struct Fetch;

impl JSApiSet for Fetch {
    fn register(&self, runtime: &javy::Runtime, _config: &crate::APIConfig) -> Result<()> {
        let context = runtime.context();
        let global = context.global_object()?;
        global.set_property("arakoo", context.value_from_bool(true)?)?;

        global.set_property(
            "__internal_http_send",
            context.wrap_callback(
                |context: &JSContextRef, _this: JSValueRef<'_>, args: &[JSValueRef<'_>]| {
                    send_http_request(context, &_this, args)
                },
            )?,
        )?;

        Ok(())
    }
}

fn send_http_request(context: &JSContextRef, _this: &JSValueRef, args: &[JSValueRef]) -> Result<JSValue> {
    match args {
        [request] => {
            let deserializer = &mut Deserializer::from(request.clone());
            let request = HttpRequest::deserialize(deserializer)?;

            let mut builder = request::Builder::new()
                .method(request.method.deref())
                .uri(request.uri.deref());

            if let Some(headers) = builder.headers_mut() {
                for (key, value) in &request.headers {
                    headers.insert(
                        HeaderName::from_bytes(key.as_bytes())?,
                        HeaderValue::from_bytes(value.as_bytes())?,
                    );
                }
            }

            let outbound_request = builder.body(request.body.map(|buffer| buffer.into_vec().into())).unwrap();

            let response = outbound_http::send_request(
                outbound_request
            )?;

            let response = HttpResponse {
                status: response.status().as_u16(),
                headers: response
                    .headers()
                    .iter()
                    .map(|(key, value)| {
                        Ok((
                            key.as_str().to_owned(),
                            str::from_utf8(value.as_bytes())?.to_owned(),
                        ))
                    })
                    .collect::<Result<_>>()?,
                body: response
                    .into_body()
                    .map(|bytes| ByteBuf::from(bytes.deref())),
                status_text: response.status().canonical_reason().unwrap_or("").to_owned(),
            };

            let mut serializer = Serializer::from_context(context)?;
            response.serialize(&mut serializer)?;
            Ok(serializer.value)
        }

        _ => Err(anyhow!("expected 1 argument, got {}", args.len())),
    }
}

