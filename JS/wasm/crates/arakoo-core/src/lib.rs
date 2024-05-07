use anyhow::anyhow;
use anyhow::Result;
use javy::quickjs::from_qjs_value;
use javy::quickjs::to_qjs_value;
use javy::quickjs::Deserializer;
use javy::quickjs::JSContextRef;
use javy::quickjs::JSValue;
use javy::quickjs::JSValueRef;
use javy::Runtime;
use once_cell::sync::OnceCell;
use send_wrapper::SendWrapper;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::ops::Deref;
use std::sync::Mutex;

use serde_bytes::ByteBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub uri: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Option<ByteBuf>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<ByteBuf>,
    pub status_text: String,

}

pub mod wit {
    use wit_bindgen::generate;

    generate!({
        path:"../../wit",
        world:"reactor",
    });

    use super::Guest;
    export!(Guest);
    pub use self::arakoo::edgechains::http_types::{Method, Request, Response};
    pub use self::exports::arakoo::edgechains::inbound_http;
    pub use self::arakoo::edgechains;
}


struct Guest;

// mod execution;
mod runtime;
mod apis;

// const FUNCTION_MODULE_NAME: &str = "function.mjs";

// static mut COMPILE_SRC_RET_AREA: [u32; 2] = [0; 2];
// static mut RUNTIME: OnceCell<Runtime> = OnceCell::new();
static CONTEXT: OnceCell<SendWrapper<&JSContextRef>> = OnceCell::new();
static HANDLER: OnceCell<SendWrapper<JSValueRef>> = OnceCell::new();
static GLOBAL: OnceCell<SendWrapper<JSValueRef>> = OnceCell::new();
static mut RUNTIME_INSTANCE: Option<Runtime> = None;
static ON_RESOLVE: OnceCell<SendWrapper<JSValueRef>> = OnceCell::new();
static ON_REJECT: OnceCell<SendWrapper<JSValueRef>> = OnceCell::new();
static RESPONSE: Mutex<Option<JSValue>> = Mutex::new(None);
static EXCEPTION: Mutex<Option<JSValue>> = Mutex::new(None);

fn on_resolve(context: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]) -> Result<JSValue> {
    // (*args).clone_into(&mut cloned_args);
    let mut qjs_value = Option::None;
    if args.len() > 0 {
        for arg in args {
            qjs_value = Some(from_qjs_value(*arg).unwrap());
            // println!("Arg resolve: {:?}", qjs_value.as_ref().unwrap());
        }
        RESPONSE.lock().unwrap().replace(qjs_value.unwrap());
        Ok(JSValue::Undefined)
    } else {
        Err(anyhow!("expected 1 argument, got {}", args.len()))
    }
}

fn on_reject(context: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]) -> Result<JSValue> {
    // (*args).clone_into(&mut cloned_args);
    let mut qjs_value = Option::None;
    if (args.len() > 0) {
        for arg in args {
            qjs_value = Some(from_qjs_value(*arg).unwrap());
            println!("Arg reject : {:?}", qjs_value.as_ref().unwrap());
        }
        EXCEPTION.lock().unwrap().replace(qjs_value.unwrap());
        Ok(JSValue::Undefined)
    } else {
        Err(anyhow!("expected 1 argument, got {}", args.len()))
    }
}

/// Used by Wizer to preinitialize the module
#[export_name = "wizer.initialize"]
pub extern "C" fn init() {
    let mut contents = String::new();
    let res = io::stdin().read_to_string(&mut contents);
    match res {
        Ok(len) => println!("Read {} bytes", len),
        Err(err) => println!(
            "Error : no input file specified or corrupted input js file supplied \n{}",
            err
        ),
    }
    // println!("Contents : {}",contents);
    unsafe {
        if RUNTIME_INSTANCE.is_none() {
            RUNTIME_INSTANCE = Some(runtime::new_runtime().unwrap());
        }
    }
    let runtime = unsafe { RUNTIME_INSTANCE.as_ref().unwrap() };
    let context = runtime.context();
    CONTEXT.set(SendWrapper::new(context)).unwrap();
    match context.eval_global("javascriptCode.js", &contents) {
        Ok(_) => (),
        Err(err) => println!("Error in evaluating script function.js : {:?}", err),
    };

    let global = context
        .global_object()
        .expect("Unable to get global object");
    GLOBAL.set(SendWrapper::new(global)).unwrap();

    let hono = global.get_property("_export").unwrap();
    let handle_event = hono.get_property("fetch").expect("Hono app not exported");
    HANDLER.set(SendWrapper::new(handle_event)).unwrap();

    let on_resolve = context.wrap_callback(on_resolve).unwrap();
    ON_RESOLVE.set(SendWrapper::new(on_resolve)).unwrap();
    let on_reject = context.wrap_callback(on_reject).unwrap();
    ON_REJECT.set(SendWrapper::new(on_reject)).unwrap();
}

impl wit::inbound_http::Guest for Guest {
    fn handle_request(req: wit::Request) -> wit::Response {
        println!("{:?}", req);
        let context = **CONTEXT.get().unwrap();
        let mut serializer =
            javy::quickjs::Serializer::from_context(context).expect("Unable to create serializer");
        let handler = **HANDLER.get().unwrap();
        let request = HttpRequest {
            method: match req.method {
                wit::Method::Get => "GET".to_string(),
                wit::Method::Post => "POST".to_string(),
                wit::Method::Put => "PUT".to_string(),
                wit::Method::Delete => "DELETE".to_string(),
                wit::Method::Patch => "PATCH".to_string(),
                wit::Method::Head => "HEAD".to_string(),
                wit::Method::Options => "OPTIONS".to_string(),
            },
            uri: req.uri,
            headers: req
                .headers
                .iter()
                .map(|(k, v)| Ok((k.as_str().to_owned(), v.as_str().to_owned())))
                .collect::<Result<HashMap<String, String>>>()
                .unwrap(),
            params: req
                .params
                .iter()
                .map(|(k, v)| Ok((k.as_str().to_owned(), v.as_str().to_owned())))
                .collect::<Result<HashMap<String, String>>>()
                .unwrap(),
            body: req.body.map(|bytes| ByteBuf::from::<Vec<u8>>(bytes)),
        };
        // let hono_event =
        // hono_event.serialize(&mut serializer).unwrap();
        request
            .serialize(&mut serializer)
            .expect("unable to serialize httprequest");
        let request_value = serializer.value;
        // println!("body of httpRequest : {:?}", from_qjs_value(request_value).unwrap());
        let global =  GLOBAL.get().unwrap() ;
        let request_to_event = global
            .get_property("requestToEvent")
            .expect("Unable to get requestToEvent");
        let event = request_to_event
            .call(global, &[request_value])
            .expect("Unable to call requestToEvent");
        let event_request = event
            .get_property("request")
            .expect("Unable to get request from event");
        let promise = handler
            .call(global, &[event_request, event])
            .expect("Unable to call handler");

        let on_resolve =  ON_RESOLVE.get().unwrap().clone() ;
        let on_reject =  ON_REJECT.get().unwrap().clone() ;
        let then_func = promise.get_property("then").unwrap();
        if then_func.is_function() {
            then_func
                .call(
                    &promise,
                    &[on_resolve.deref().clone(), on_reject.deref().clone()],
                )
                .unwrap();
        } else {
            RESPONSE
                .lock()
                .unwrap()
                .replace(from_qjs_value(promise).unwrap());
        }

        context
            .execute_pending()
            .expect("Unable to execute pending tasks");

        // let response = to_qjs_value(context, &RESPONSE.lock().unwrap().take().unwrap()).unwrap();
        let response = RESPONSE.lock().unwrap().take().unwrap();

        // let deserializer = &mut Deserializer::from(response);
        // let response = HttpResponse::deserialize(deserializer).unwrap();
        // println!("Http Response {:?}", response);
        if let JSValue::Object(obj) = response {
            let status_code_ref = to_qjs_value(context, obj.get("status").unwrap()).unwrap();
            let status_code = status_code_ref.as_i32_unchecked();
            let status_text_ref = to_qjs_value(context, obj.get("statusText").unwrap()).unwrap();
            let status_text = status_text_ref.as_str().unwrap();
            let body_ref = to_qjs_value(context, obj.get("body").unwrap()).unwrap();
            let headers = obj.get("headers").unwrap();
            let mut headers_vec = Vec::new();
            let headers_obj = match headers {
                JSValue::Object(obj) => obj,
                _ => panic!("Headers is not object {:?}", headers),
            };
            if let JSValue::Object(headers_obj) = headers_obj.get("headers").unwrap() {
                for (k, v) in headers_obj.iter() {
                    let key = k.clone();
                    let value = (*v).to_string();
                    headers_vec.push((key.to_string(), value.to_string()));
                }
            }
            wit::Response {
                status: status_code as u16,
                headers: Some(headers_vec),
                body: Some(body_ref.as_str().unwrap().as_bytes().to_vec()),
                status_text: status_text.to_string(),
            }
        } else {
            panic!("Response is not object {:?}", response);
        }
    }
}

/// Compiles JS source code to QuickJS bytecode.
///
/// Returns a pointer to a buffer containing a 32-bit pointer to the bytecode byte array and the
/// u32 length of the bytecode byte array.
///
/// # Arguments
///
/// * `js_src_ptr` - A pointer to the start of a byte array containing UTF-8 JS source code
/// * `js_src_len` - The length of the byte array containing JS source code
///
/// # Safety
///
/// * `js_src_ptr` must reference a valid array of unsigned bytes of `js_src_len` length
// #[export_name = "compile_src"]
// pub unsafe extern "C" fn compile_src(js_src_ptr: *const u8, js_src_len: usize) -> *const u32 {
//     // Use fresh runtime to avoid depending on Wizened runtime
//     let runtime = runtime::new_runtime().unwrap();
//     let js_src = str::from_utf8(slice::from_raw_parts(js_src_ptr, js_src_len)).unwrap();
//     let bytecode = runtime
//         .context()
//         .compile_module(FUNCTION_MODULE_NAME, js_src)
//         .unwrap();
//     let bytecode_len = bytecode.len();
//     // We need the bytecode buffer to live longer than this function so it can be read from memory
//     let bytecode_ptr = Box::leak(bytecode.into_boxed_slice()).as_ptr();
//     COMPILE_SRC_RET_AREA[0] = bytecode_ptr as u32;
//     COMPILE_SRC_RET_AREA[1] = bytecode_len.try_into().unwrap();
//     COMPILE_SRC_RET_AREA.as_ptr()
// }

/// Evaluates QuickJS bytecode
///
/// # Safety
///
/// * `bytecode_ptr` must reference a valid array of unsigned bytes of `bytecode_len` length
// #[export_name = "eval_bytecode"]
// pub unsafe extern "C" fn eval_bytecode(bytecode_ptr: *const u8, bytecode_len: usize) {
//     let runtime = RUNTIME.get().unwrap();
//     let bytecode = slice::from_raw_parts(bytecode_ptr, bytecode_len);
//     execution::run_bytecode(runtime, bytecode);
// }

/// Evaluates QuickJS bytecode and invokes the exported JS function name.
///
/// # Safety
///
/// * `bytecode_ptr` must reference a valid array of bytes of `bytecode_len`
///   length.
/// * `fn_name_ptr` must reference a UTF-8 string with `fn_name_len` byte
///   length.
#[export_name = "invoke"]
pub unsafe extern "C" fn invoke(
    bytecode_ptr: *const u8,
    bytecode_len: usize,
    fn_name_ptr: *const u8,
    fn_name_len: usize,
) {
    // let runtime = RUNTIME.get().unwrap();
    // let bytecode = slice::from_raw_parts(bytecode_ptr, bytecode_len);
    // let fn_name = str::from_utf8_unchecked(slice::from_raw_parts(fn_name_ptr, fn_name_len));
    // execution::run_bytecode(runtime, bytecode);
    // execution::invoke_function(runtime, FUNCTION_MODULE_NAME, fn_name);
}
