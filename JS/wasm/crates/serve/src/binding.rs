use super::http_types::{Headers, HttpError, Method, Response};
use async_trait::async_trait;
use http::HeaderMap;
use reqwest::Url;

// use std::{
//     env,
//     sync::{Arc, Mutex},
// };

// use arakoo_jsonnet::{
//     ext_string, jsonnet_destroy, jsonnet_evaluate_file, jsonnet_evaluate_snippet, jsonnet_make,
// };
use jrsonnet_evaluator::{
    function::{
        builtin::{NativeCallback, NativeCallbackHandler},
        TlaArg,
    },
    gc::GcHashMap,
    manifest::ManifestFormat,
    trace::TraceFormat,
    State, Val,
};
use jrsonnet_parser::IStr;
use tokio::runtime::Handle;

use std::{fs, io, ops::DerefMut};
// use tokio::runtime::Builder;
use tracing::{debug, error};
// use wasmtime::*;

use crate::io::{WasmInput, WasmOutput};

#[derive(jrsonnet_gcmodule::Trace)]
pub struct NativeJSCallback(String);

impl NativeCallbackHandler for NativeJSCallback {
    fn call(
        &self,
        args: &[jrsonnet_evaluator::Val],
    ) -> jrsonnet_evaluator::Result<jrsonnet_evaluator::Val> {
        // unsafe {
        //     reactor = super::REACTOR.clone().expect("Component not instantiated");
        // }
        // let store_ref;
        // unsafe {
        //     store_ref = super::STORE.clone().expect("Store not instantiated");
        // }
        // let reactor = unsafe { super::REACTOR.as_ref() };
        // let store = unsafe { super::STORE.as_mut() };
        // let reactor = reactor.expect("Reactor not initialized");
        // let store = store.expect("Store not initialized");
        // let output = tokio::runtime::Runtime::new()
        //     .unwrap()
        //     .block_on(async {
        //         // let mut store = store_ref.lock().await;
        //         let jsonnet_guest = reactor.arakoo_edgechains_jsonnet_func();
        //         println!("Calling guest function");
        //         let result = jsonnet_guest.call_jsonnet_call_native_func(store, &self.0, "").await;
        //         result
        //     })
        //     .expect("Unable to call function from jsonnet on guest");
        // println!("output from guest : {}", &output);
        unsafe {
            let handle = Handle::current();
            let result = tokio::task::block_in_place(|| {
                handle.block_on(async {
                    debug!("Acquiring lock of reactor");
                    // let reactor = super::REACTOR
                    //     .as_ref()
                    //     .expect("Unable to get reactor option")
                    //     .lock()
                    //     .await;
                    let reactor = super::REACTOR.clone().unwrap();
                    debug!("Acquiring lock of store");
                    // let mut store = super::STORE
                    //     .as_ref()
                    //     .expect("Unable to get store option")
                    //     .lock()
                    //     .await;
                    // let wasi = super::WasiCtxBuilder::new()
                    //     .inherit_stdout()
                    //     .inherit_stderr()
                    //     .build();

                    // let table = super::ResourceTable::new();
                    // let mut store = super::Store::new(
                    //     super::WORKER_CTX.as_ref().unwrap().clone().engine(),
                    //     super::Host {
                    //         table,
                    //         wasi,
                    //         client: None,
                    //     },
                    // );
                    let store = super::STORE.as_mut().unwrap();
                    debug!("Calling guest native function");
                    let guest = reactor.arakoo_edgechains_jsonnet_func();
                    let result = guest
                        .call_jsonnet_call_native_func(&mut *store, &self.0, "")
                        .await
                        .expect("Unable to call function from jsonnet on guest");
                    debug!("Result from guest : {:?}", &result);
                    result
                })
            });
            Ok(Val::Str(result.into()))
        }
    }
}

#[async_trait]
impl super::jsonnet::Host for super::Host {
    async fn jsonnet_make(&mut self) -> wasmtime::Result<u64> {
        let ptr = arakoo_jsonnet::jsonnet_make();
        Ok(ptr as u64)
    }

    async fn jsonnet_evaluate_snippet(
        &mut self,
        vm: u64,
        file: String,
        code: String,
    ) -> wasmtime::Result<String> {
        let out =
            arakoo_jsonnet::jsonnet_evaluate_snippet(vm as *mut arakoo_jsonnet::VM, &file, &code);
        Ok(out)
    }

    async fn jsonnet_evaluate_file(&mut self, vm: u64, path: String) -> wasmtime::Result<String> {
        let code = fs::read_to_string(&path).map_err(|e| {
            error!("Failed to read file {}: {}", path, e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;
        debug!("Code snippet in file : {}", &code);
        let out = arakoo_jsonnet::jsonnet_evaluate_snippet(
            vm as *mut arakoo_jsonnet::VM,
            "snippet",
            &code,
        );
        debug!("Out : {}", &out);
        Ok(out)
    }

    async fn jsonnet_ext_string(
        &mut self,
        vm: u64,
        key: String,
        value: String,
    ) -> wasmtime::Result<()> {
        arakoo_jsonnet::jsonnet_ext_string(vm as *mut arakoo_jsonnet::VM, &key, &value);
        Ok(())
    }

    async fn jsonnet_register_func(
        &mut self,
        vmptr: u64,
        func_name: String,
        args_num: u32,
    ) -> wasmtime::Result<()> {
        let vm = vmptr as *mut arakoo_jsonnet::VM;
        let vm = unsafe { &*vm };
        let any_resolver = vm.state.context_initializer();
        let args_vec = vec![String::from("x"); args_num as usize];
        any_resolver
            .as_any()
            .downcast_ref::<arakoo_jsonnet::context::ArakooContextInitializer>()
            .expect("only arakoo context initializer supported")
            .add_native(
                func_name.clone(),
                NativeCallback::new(args_vec, NativeJSCallback(func_name)),
            );
        Ok(())
    }

    async fn jsonnet_destroy(&mut self, vm: u64) -> wasmtime::Result<()> {
        arakoo_jsonnet::jsonnet_destroy(vm as *mut arakoo_jsonnet::VM);
        Ok(())
    }
}

// Bindings for jsonnet

#[async_trait]
impl super::outbound_http::Host for super::Host {
    async fn send_request(
        &mut self,
        req: super::http_types::Request,
    ) -> wasmtime::Result<Result<super::http_types::Response, super::http_types::HttpError>> {
        // println!("Sending request: {:?}", request);
        Ok(async {
            tracing::log::trace!("Attempting to send outbound HTTP request to {}", req.uri);

            let method = method_from(req.method);
            let url = Url::parse(&req.uri).map_err(|_| HttpError::InvalidUrl)?;
            let headers = request_headers(req.headers).map_err(|_| HttpError::RuntimeError)?;
            let body = req.body.unwrap_or_default().to_vec();

            if !req.params.is_empty() {
                tracing::log::warn!("HTTP params field is deprecated");
            }

            // Allow reuse of Client's internal connection pool for multiple requests
            // in a single component execution
            let client = self.client.get_or_insert_with(Default::default);

            let resp = client
                .request(method, url)
                .headers(headers)
                .body(body)
                .send()
                .await
                .map_err(log_reqwest_error)?;
            tracing::log::trace!("Returning response from outbound request to {}", req.uri);
            response_from_reqwest(resp).await
        }
        .await)
    }
}

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
