// use std::{
//     env,
//     sync::{Arc, Mutex},
// };

// use arakoo_jsonnet::{
//     ext_string, jsonnet_destroy, jsonnet_evaluate_file, jsonnet_evaluate_snippet, jsonnet_make,
// };

// use std::{fs, io};
// use tokio::runtime::Builder;
// use tracing::error;
// use wasmtime::*;

// use crate::io::{WasmInput, WasmOutput};

// // pub struct VM {
// //     state: State,
// //     manifest_format: Box<dyn ManifestFormat>,
// //     trace_format: Box<dyn TraceFormat>,
// //     tla_args: GcHashMap<IStr, TlaArg>,
// // }

// /// Adds exported functions to the Wasm linker.
// ///
// /// This function wraps the `jsonnet_evaluate`, `jsonnet_output_len`, and `jsonnet_output`
// /// functions to be called from WebAssembly. It sets up the necessary state and
// /// memory management for evaluating Jsonnet code and writing the output back to
// /// WebAssembly memory.
// pub fn add_jsonnet_to_linker(linker: &mut Linker<WasiCtx>) -> anyhow::Result<()> {
//     // Create a shared output buffer that will be used to store the result of the Jsonnet evaluation.
//     let output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
//     let mut output_clone = output.clone();

//     // Wrap the `jsonnet_evaluate` function to be called from WebAssembly.
//     linker.func_wrap(
//         "arakoo",
//         "jsonnet_evaluate",
//         move |mut caller: Caller<'_, WasiCtx>,
//               var_ptr: i32,
//               var_len: i32,
//               path_ptr: i32,
//               code_len: i32| {
//             // Clone the output buffer for use within the closure.
//             let output = output_clone.clone();
//             // Get the WebAssembly memory instance.
//             let mem = match caller.get_export("memory") {
//                 Some(Extern::Memory(mem)) => mem,
//                 _ => return Err(Trap::NullReference.into()),
//             };
//             // Calculate the offsets for the variable and path buffers in WebAssembly memory.
//             let var_offset = var_ptr as u32 as usize;
//             let path_offset = path_ptr as u32 as usize;
//             // Create buffers to read the variable and path data from WebAssembly memory.
//             let mut var_buffer = vec![0; var_len as usize];
//             let mut path_buffer = vec![0; code_len as usize];

//             // Read the path data from WebAssembly memory and convert it to a string.
//             let path = match mem.read(&caller, path_offset, &mut path_buffer) {
//                 Ok(_) => match std::str::from_utf8(&path_buffer) {
//                     Ok(s) => s,
//                     Err(_) => return Err(Trap::BadSignature.into()),
//                 },
//                 _ => return Err(Trap::MemoryOutOfBounds.into()),
//             };
//             // Read the variable data from WebAssembly memory and convert it to a string.
//             let var = match mem.read(&caller, var_offset, &mut var_buffer) {
//                 Ok(_) => match std::str::from_utf8(&var_buffer) {
//                     Ok(s) => s,
//                     Err(_) => return Err(Trap::BadSignature.into()),
//                 },
//                 _ => return Err(Trap::MemoryOutOfBounds.into()),
//             };
//             // Parse the variable data as JSON.
//             let var_json: serde_json::Value = match serde_json::from_str(var) {
//                 Ok(v) => v,
//                 Err(e) => {
//                     error!("Error parsing var: {}", e);
//                     return Err(Trap::BadSignature.into());
//                 }
//             };

//             // Initialize the Jsonnet VM state with default settings.
//             let vm = jsonnet_make();

//             // Evaluate the Jsonnet code snippet using the provided path and variables.
//             let code = path;
//             for (key, value) in var_json.as_object().unwrap() {
//                 // context.add_ext_var(key.into(), Val::Str(value.as_str().unwrap().into()));
//                 ext_string(
//                     vm,
//                     key,
//                     value.as_str().expect("ext_string value is not a string"),
//                 );
//             }
//             let out = jsonnet_evaluate_snippet(vm, "deleteme", code);
//             // Store the output of the Jsonnet evaluation in the shared output buffer.
//             let mut output = output.lock().unwrap();
//             *output = out;
//             jsonnet_destroy(vm);
//             Ok(())
//         },
//     )?;

//     output_clone = output.clone();
//     // Wrap the `jsonnet_evaluate_file` function to be called from WebAssembly.
//     linker.func_wrap(
//         "arakoo",
//         "jsonnet_evaluate_file",
//         move |mut caller: Caller<'_, WasiCtx>,
//               var_ptr: i32,
//               var_len: i32,
//               path_ptr: i32,
//               code_len: i32| {
//             // Clone the output buffer for use within the closure.
//             let output = output_clone.clone();
//             // Get the WebAssembly memory instance.
//             let mem = match caller.get_export("memory") {
//                 Some(Extern::Memory(mem)) => mem,
//                 _ => return Err(Trap::NullReference.into()),
//             };
//             // Calculate the offsets for the variable and path buffers in WebAssembly memory.
//             let var_offset = var_ptr as u32 as usize;
//             let path_offset = path_ptr as u32 as usize;
//             // Create buffers to read the variable and path data from WebAssembly memory.
//             let mut var_buffer = vec![0; var_len as usize];
//             let mut path_buffer = vec![0; code_len as usize];

//             // Read the path data from WebAssembly memory and convert it to a string.
//             let path = match mem.read(&caller, path_offset, &mut path_buffer) {
//                 Ok(_) => match std::str::from_utf8(&path_buffer) {
//                     Ok(s) => s,
//                     Err(_) => return Err(Trap::BadSignature.into()),
//                 },
//                 _ => return Err(Trap::MemoryOutOfBounds.into()),
//             };
//             // Read the variable data from WebAssembly memory and convert it to a string.
//             let var = match mem.read(&caller, var_offset, &mut var_buffer) {
//                 Ok(_) => match std::str::from_utf8(&var_buffer) {
//                     Ok(s) => s,
//                     Err(_) => return Err(Trap::BadSignature.into()),
//                 },
//                 _ => return Err(Trap::MemoryOutOfBounds.into()),
//             };
//             // Parse the variable data as JSON.
//             let var_json: serde_json::Value = match serde_json::from_str(var) {
//                 Ok(v) => v,
//                 Err(e) => {
//                     error!("Error parsing var: {}", e);
//                     return Err(Trap::BadSignature.into());
//                 }
//             };
//             // println!("var_json: {:?}", var_json);

//             let vm = jsonnet_make();

//             for (key, value) in var_json.as_object().unwrap() {
//                 ext_string(
//                     vm,
//                     key,
//                     value.as_str().expect("ext_string value is not a string"),
//                 );
//             }
//             let code = fs::read_to_string(path).expect("File not found");
//             let out = jsonnet_evaluate_snippet(vm, "deleteme", &code);
//             let mut output: std::sync::MutexGuard<'_, String> = output.lock().unwrap();
//             *output = out;

//             Ok(())
//         },
//     )?;

//     // Wrap the `jsonnet_output_len` function to be called from WebAssembly.
//     // This function returns the length of the output string.
//     let output_clone = output.clone();
//     linker.func_wrap("arakoo", "jsonnet_output_len", move || -> i32 {
//         let output_clone = output_clone.clone();
//         let output = output_clone.lock().unwrap().clone();
//         output.len() as i32
//     })?;

//     // Wrap the `jsonnet_output` function to be called from WebAssembly.
//     // This function writes the output string to the specified memory location.
//     linker.func_wrap(
//         "arakoo",
//         "jsonnet_output",
//         move |mut caller: Caller<'_, WasiCtx>, ptr: i32| {
//             let output_clone = output.clone();
//             let mem = match caller.get_export("memory") {
//                 Some(Extern::Memory(mem)) => mem,
//                 _ => return Err(Trap::NullReference.into()),
//             };
//             let offset = ptr as u32 as usize;
//             let out = output_clone.lock().unwrap().clone();
//             match mem.write(&mut caller, offset, out.as_bytes()) {
//                 Ok(_) => {}
//                 _ => return Err(Trap::MemoryOutOfBounds.into()),
//             };
//             Ok(())
//         },
//     )?;

//     Ok(())
// }

use super::http_types::{Headers, HttpError, Method, Response};
use http::HeaderMap;

pub fn log_reqwest_error(err: reqwest::Error) -> HttpError {
    let error_desc = if err.is_timeout() {
        "timeout error"
    } else if err.is_connect() {
        "connection error"
    } else if err.is_body() || err.is_decode() {
        "message body error"
    } else if err.is_request() {
        "request error"
    } else {
        "error"
    };
    tracing::warn!(
        "Outbound HTTP {}: URL {}, error detail {:?}",
        error_desc,
        err.url()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "<unknown>".to_owned()),
        err
    );
    HttpError::RuntimeError
}

pub fn method_from(m: Method) -> http::Method {
    match m {
        Method::Get => http::Method::GET,
        Method::Post => http::Method::POST,
        Method::Put => http::Method::PUT,
        Method::Delete => http::Method::DELETE,
        Method::Patch => http::Method::PATCH,
        Method::Head => http::Method::HEAD,
        Method::Options => http::Method::OPTIONS,
    }
}

pub async fn response_from_reqwest(res: reqwest::Response) -> Result<Response, HttpError> {
    let status = res.status().as_u16();
    let headers = response_headers(res.headers()).map_err(|_| HttpError::RuntimeError)?;
    let status_text = (&res.status().canonical_reason().unwrap_or("")).to_string();
    let body = Some(
        res.bytes()
            .await
            .map_err(|_| HttpError::RuntimeError)?
            .to_vec(),
    );

    Ok(Response {
        status,
        headers,
        body,
        status_text,
    })
}

pub fn request_headers(h: Headers) -> anyhow::Result<HeaderMap> {
    let mut res = HeaderMap::new();
    for (k, v) in h {
        res.insert(
            http::header::HeaderName::try_from(k)?,
            http::header::HeaderValue::try_from(v)?,
        );
    }
    Ok(res)
}

pub fn response_headers(h: &HeaderMap) -> anyhow::Result<Option<Vec<(String, String)>>> {
    let mut res: Vec<(String, String)> = vec![];

    for (k, v) in h {
        res.push((
            k.to_string(),
            std::str::from_utf8(v.as_bytes())?.to_string(),
        ));
    }

    Ok(Some(res))
}
