pub mod types;

use std::collections::HashMap;

use anyhow::Result;
use javy::quickjs::{JSContextRef, JSValue, JSValueRef};

use crate::{fetch, get_response, get_response_len, http::types::Request, JSApiSet};
pub(super) struct Http;

impl JSApiSet for Http {
    fn register(&self, runtime: &javy::Runtime, _config: &crate::APIConfig) -> Result<()> {
        let context = runtime.context();
        context.eval_global("http.js", include_str!("shims/dist/index.js"))?;
        let global = context.global_object()?;
        global.set_property("arakoo", context.value_from_bool(true)?)?;
        global.set_property("fetch_internal", context.wrap_callback(fetch_callback())?)?;
        // global.set_property(
        //     "__internal_http_send",
        //     context.wrap_callback(
        //         |context: &JSContextRef, _this: JSValueRef<'_>, args: &[JSValueRef<'_>]| {
        //             send_http_request(context, &_this, args)
        //         },
        //     )?,
        // )?;
        Ok(())
    }
}

fn fetch_callback(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx, _this, args| {
        if args.len() < 1 {
            return Err(anyhow::anyhow!(
                "Expected at least 1 argument, got {}",
                args.len()
            ));
        }
        let uri = args.get(0).unwrap().to_string();
        let opts: HashMap<String, JSValue> = args[1].try_into()?;
        let method = opts.get("method").unwrap_or(&"GET".into()).to_string();
        let headers = match opts.get("headers") {
            Some(JSValue::Object(headers)) => headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            _ => HashMap::default(),
        };
        let body = opts.get("body").unwrap_or(&"".into()).to_string();
        let params = match opts.get("params") {
            Some(JSValue::Object(params)) => params
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            _ => HashMap::default(),
        };

        let request =
            serde_json::to_string(&Request::new(uri, method, headers, body, params)).unwrap();
        let request_ptr = request.as_ptr();
        let request_len = request.len() as i32;
        unsafe { fetch(request_ptr, request_len) }
        let response_len = unsafe { get_response_len() };
        let mut response_buffer = Vec::with_capacity(response_len as usize);
        let response_ptr = response_buffer.as_mut_ptr();
        let response_buffer = unsafe {
            get_response(response_ptr);
            Vec::from_raw_parts(response_ptr, response_len as usize, response_len as usize)
        };
        let response: serde_json::Value = match serde_json::from_slice(&response_buffer) {
            Ok(response) => response,
            Err(e) => {
                eprintln!("Failed to parse fetch response: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to parse fetch response: {}",
                    e.to_string()
                ));
            }
        };
        let mut headers_map: HashMap<String, JSValue> = HashMap::new();
        for (key, value) in response["headers"].as_object().unwrap() {
            headers_map.insert(
                key.to_string(),
                JSValue::String(value.as_str().unwrap().to_string()),
            );
        }
        let mut response_map: HashMap<String, JSValue> = HashMap::new();
        response_map.insert(
            "status".to_string(),
            JSValue::Int(response["status"].as_i64().unwrap().try_into().unwrap()),
        );
        response_map.insert(
            "statusText".to_string(),
            JSValue::String(response["statusText"].as_str().unwrap().to_string()),
        );
        response_map.insert(
            "body".to_string(),
            JSValue::String(response["body"].as_str().unwrap().to_string()),
        );
        response_map.insert("headers".to_string(), JSValue::Object(headers_map));
        let response_obj = JSValue::Object(response_map);
        // todo!("fetch");
        Ok(response_obj)
    }
}

// fn send_http_request(context: &Context, _this: &Value, args: &[Value]) -> Result<Value> {
//     match args {
//         [request] => {
//             let deserializer = &mut Deserializer::from(request.clone());
//             let request = HttpRequest::deserialize(deserializer)?;

//             let mut builder = request::Builder::new()
//                 .method(request.method.deref())
//                 .uri(request.uri.deref());

//             if let Some(headers) = builder.headers_mut() {
//                 for (key, value) in &request.headers {
//                     headers.insert(
//                         HeaderName::from_bytes(key.as_bytes())?,
//                         HeaderValue::from_bytes(value.as_bytes())?,
//                     );
//                 }
//             }

//             let response = arakoo_send_request(
//                 builder.body(request.body.map(|buffer| buffer.into_vec().into()))?,
//             )?;

//             let response = HttpResponse {
//                 status: response.status().as_u16(),
//                 headers: response
//                     .headers()
//                     .iter()
//                     .map(|(key, value)| {
//                         Ok((
//                             key.as_str().to_owned(),
//                             str::from_utf8(value.as_bytes())?.to_owned(),
//                         ))
//                     })
//                     .collect::<Result<_>>()?,
//                 body: response
//                     .into_body()
//                     .map(|bytes| ByteBuf::from(bytes.deref())),
//             };

//             let mut serializer = Serializer::from_context(context)?;
//             response.serialize(&mut serializer)?;
//             Ok(serializer.value)
//         }

//         _ => Err(anyhow!("expected 1 argument, got {}", args.len())),
//     }
// }

// pub fn arakoo_send_request(req: HttpRequest) -> Result<HttpResponse> {
//     let (req, body) = req.into_parts();

//     let method = req.method.try_into()?;

//     let uri = req.uri.to_string();

//     let params = vec![];

//     let headers = &req
//         .headers
//         .iter()
//         .map(try_header_to_strs)
//         .collect::<Result<Vec<_>>>()?;

//     let body = body.as_ref().map(|bytes| bytes.as_ref());

//     let out_req = OutboundRequest {
//         method,
//         uri: &uri,
//         params: &params,
//         headers,
//         body,
//     };

//     let OutboundResponse {
//         status,
//         headers,
//         body,
//     } = spin_http::send_request(out_req)?;

//     let resp_builder = http_types::response::Builder::new().status(status);
//     let resp_builder = headers
//         .into_iter()
//         .flatten()
//         .fold(resp_builder, |b, (k, v)| b.header(k, v));
//     resp_builder
//         .body(body.map(Into::into))
//         .map_err(|_| OutboundHttpError::RuntimeError)
// }