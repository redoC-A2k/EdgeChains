use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::HashMap,
    future::IntoFuture,
    path::Path,
    process::Output,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use futures::{Future, FutureExt};
use jrsonnet_evaluator::{
    apply_tla,
    error::ErrorKind,
    function::{
        builtin::{self, NativeCallback, NativeCallbackHandler},
        TlaArg,
    },
    gc::GcHashMap,
    manifest::{JsonFormat, ManifestFormat},
    tb,
    trace::{CompactFormat, PathResolver, TraceFormat},
    FileImportResolver, ObjValueBuilder, State, Thunk, Val,
};
use jrsonnet_gcmodule::Trace;
use jrsonnet_parser::IStr;
use js_sys::Promise;
use std::alloc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

mod context;
extern crate console_error_panic_hook;
use std::panic;

#[wasm_bindgen(module = "/read-file.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn read_file(path: &str) -> Result<String, JsValue>;
}

// console log
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub struct VM {
    state: State,
    manifest_format: Box<dyn ManifestFormat>,
    trace_format: Box<dyn TraceFormat>,
    tla_args: GcHashMap<IStr, TlaArg>,
}

pub struct NativeContext {
    // pub vm: &'a VM,
    pub vm: *mut VM,
}

pub struct MapStruct {
    pub func_map: HashMap<String, js_sys::Function>,
}

static mut map_struct: Option<MapStruct> = None;

#[wasm_bindgen]
pub fn jsonnet_make() -> *mut VM {
    unsafe {
        map_struct = Some(MapStruct {
            func_map: HashMap::new(),
        });
    }
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let state = State::default();
    state.settings_mut().import_resolver = tb!(FileImportResolver::default());
    state.set_context_initializer(context::ArakooContextInitializer::new(
        state.clone(),
        PathResolver::new_cwd_fallback(),
    ));
    // add_namespace(&state);
    Box::into_raw(Box::new(VM {
        state,
        manifest_format: Box::new(JsonFormat::default()),
        trace_format: Box::new(CompactFormat::default()),
        tla_args: GcHashMap::default(),
    }))
}

#[wasm_bindgen]
pub fn jsonnet_destroy(vm: *mut VM) {
    unsafe {
        let dloc_vm = Box::from_raw(vm);
        drop(dloc_vm);
    }
}

#[wasm_bindgen]
pub fn jsonnet_evaluate_snippet(vm: *mut VM, filename: &str, snippet: &str) -> String {
    let vm = unsafe { &mut *vm };
    match vm
        .state
        .evaluate_snippet(filename, snippet)
        .and_then(|val| apply_tla(vm.state.clone(), &vm.tla_args, val))
        .and_then(|val| val.manifest(&vm.manifest_format))
    {
        Ok(v) => v,
        Err(e) => {
            let mut out = String::new();
            vm.trace_format.write_trace(&mut out, &e).unwrap();
            out
        }
    }
}

#[wasm_bindgen]
pub fn jsonnet_evaluate_file(vm: *mut VM, filename: &str) -> String {
    let vm = unsafe { &mut *vm };
    log(&format!("Reading file {}", filename));
    match read_file(filename) {
        Ok(content) => match vm
            .state
            .evaluate_snippet(filename, &content)
            .and_then(|val| apply_tla(vm.state.clone(), &vm.tla_args, val))
            .and_then(|val| val.manifest(&vm.manifest_format))
        {
            Ok(v) => v,
            Err(e) => {
                let mut out = String::new();
                vm.trace_format.write_trace(&mut out, &e).unwrap();
                out
            }
        },
        Err(e) => {
            log(&format!("Error {:?}", e));
            eprintln!("Error reading file: {}", e.as_string().unwrap());
            let out = String::from(e.as_string().unwrap());
            out
        }
    }
}

#[wasm_bindgen]
pub fn ext_string(vm: *mut VM, key: &str, value: &str) {
    let vm = unsafe { &mut *vm };
    // let context_initializer_ref = vm.state.context_initializer();

    // Dereference the Ref to access the trait object
    let context_initializer = vm.state.context_initializer();

    context_initializer
        .as_any()
        .downcast_ref::<context::ArakooContextInitializer>()
        .expect("only arakoo context initializer supported")
        .add_ext_var(key.into(), Val::Str(value.into()));
}

// #[wasm_bindgen]
// pub struct CallBackClass {
//     arg: String,
//     func: js_sys::Function
// }

// #[wasm_bindgen]
// impl CallBackClass {
//     #[wasm_bindgen(constructor)]
//     pub extern "C" fn new(f: &js_sys::Function) -> CallBackClass {
//         CallBackClass {
//             arg: String::from(""),
//             func: f.clone(),
//         }
//     }

//     pub extern "C" fn call_native_js_func(&self) -> Result<JsValue, JsValue> {
//         let this = JsValue::null();
//         // for x in self.args {
//         // let x = JsValue::from(x);
//         let result = self.func.call1(&this, &JsValue::from_str(&self.arg));
//         println!("Result of calling JS function: {:?}", result);
//         return result;
//         // }
//     }

//     #[wasm_bindgen(getter)]
//     pub extern "C" fn arg(&self) -> String {
//         self.arg.clone()
//     }

//     #[wasm_bindgen(setter)]
//     pub extern "C" fn set_arg(&mut self, arg: String) {
//         self.arg = arg;
//     }
// }

#[wasm_bindgen]
pub fn get_func(name: &str) -> Option<js_sys::Function> {
    unsafe {
        let func = map_struct.as_mut().unwrap().func_map.get(name);
        match func {
            Some(f) => Some(f.clone()),
            None => None,
        }
    }
}

#[wasm_bindgen]
pub fn set_func(name: String, func: js_sys::Function) {
    unsafe {
        map_struct.as_mut().unwrap().func_map.insert(name, func);
    }
}

#[derive(jrsonnet_gcmodule::Trace)]
struct NativeJSCallback(String);

#[derive(Clone)]
struct JsValueWrap(JsValue);
#[derive(Clone)]
struct PromiseWrap(js_sys::Promise);

unsafe impl Send for JsValueWrap {}
unsafe impl Send for PromiseWrap {}

impl NativeCallbackHandler for NativeJSCallback {
    fn call(&self, args: &[Val]) -> jrsonnet_evaluator::Result<Val> {
        let this = JsValue::null();
        let func = get_func(&self.0).expect("Function not found");
        let result;
        if args.len() > 0 {
            result = func.call1(
                &this,
                &JsValue::from_str(
                    &serde_json::to_string(args).expect("Error converting args to JSON"),
                ),
            );
        } else {
            result = func.call0(&this);
        }
        if let Ok(r) = result {
            if r.clone().is_instance_of::<js_sys::Promise>() {
                let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
                let promise = r.dyn_into::<Promise>().expect("JsValue is not promise");
                let result_clone = result.clone();
                let _ = promise.then2(
                    &Closure::once(move |val: JsValue| {
                        log(&format!("Got value {:?}", val));
                        let mut x = result_clone.borrow_mut();
                        *x = Some(val.as_string().expect("Value can't be converted to String"));
                    }),
                    &Closure::once(move |error| {
                        log(&format!("Got error {:?}", error));
                    }),
                );
                let x = result.borrow_mut();
                //Fix : Thread gets blocked here 
                while(*x).is_none() {
                    // thread::sleep(Duration::from_millis(100));
                    // log("Waiting for value");
                    continue;
                }
                let output = x.clone().unwrap();
                // // wasm_bindgen_futures::JsFuture::from(promise)
                // // .into_future()
                // // .then( |val| {
                // //     log(&format!("Result of calling JS function: {:?}", val));
                // //     Ok(Val::Str("good".into()))
                // // });
                // // let (sender, recx) = mpsc::channel::<JsValueWrap>();
                // let data: Arc<Mutex<Option<JsValueWrap>>> = Arc::new(Mutex::new(None));
                // let data_clone = data.clone();
                // // let flag = Arc::new(Mutex::new(false));
                // // let promise_rc = Arc::new(PromiseWrap(promise));
                // // let promise_rc_clone = promise_rc.clone();
                // let r = JsValueWrap(r);
                // let r = Arc::new(Mutex::new(r));
                // let r = r.clone();
                // // tokio::runtime::Builder::new_current_thread()
                // //     .enable_all()
                // //     .build()
                // //     .unwrap()
                // //     .spawn_blocking(move || {
                // let r = r.lock().unwrap();
                // let r = &r.borrow().0;
                // let r = r.clone();
                // let promise = r
                //     .dyn_into::<Promise>()
                //     .expect("Unable to typecast to promise");
                // // let val:&dyn Future<Output = Result<JsValue,JsValue>> = JsFuture::from(promise).borrow();
                // // wasm_bindgen_futures::spawn_local(async move {
                // //     let val = JsFuture::from(promise).await.unwrap();
                // //     log(&format!("Result of calling JS function: {:?}", val));
                // //     let mut data = data_clone.lock().unwrap();
                // //     *data = Some(JsValueWrap(val));
                // //     // sender.send(val).expect("unable to send value over mpsc");
                // // });
                // // tokio::task::spawn_blocking(move || {
                // let data = data.lock().unwrap();
                // // while data.is_none() {
                // //     thread::sleep(Duration::from_millis(100));
                // //     log("Waiting for value");
                // //     continue;
                // // }
                // let val = data.clone().unwrap();
                // log("Recieveing value");
                // // });
                // // });

                // // let recx = receiver.
                // // let received = recx.recv().expect("Unable to recieve value over mpsc");
                // // log(&format!("Recieved {:?}", received));
                // // Ok(Val::Str(received.as_string().expect("Value can't be converted to String").into()))

                Ok(Val::Str(output.into()))
            } else {
                // If it's not a promise, return directly
                Ok(Val::Str(r.as_string().unwrap().into()))
            }
        } else {
            // If there's an error, return an error
            Err(ErrorKind::RuntimeError("Error calling JS function".into()).into())
        }
    }
}

// Function to register the native callback
#[wasm_bindgen]
pub extern "C" fn register_native_callback(vm: *mut VM, name: String, args_num: u32) {
    let vm = unsafe { &*vm };
    let any_resolver = vm.state.context_initializer();
    let args_vec = vec![String::from("x"); args_num as usize];
    any_resolver
        .as_any()
        .downcast_ref::<context::ArakooContextInitializer>()
        .expect("only arakoo context initializer supported")
        .add_native(
            name.clone(),
            NativeCallback::new(args_vec, NativeJSCallback(name)),
        );
}

#[cfg(test)]
mod test {
    use super::*;
    use regex::Regex;
    #[test]
    // #[wasm_bindgen]
    pub fn test_ext_string() {
        let vm = jsonnet_make();
        // let filename = CString::new("filename").unwrap();
        let filename = "filename";

        let snippet = r#"
    local username = std.extVar('name');
    local Person(name='Alice') = {
      name: name,
      welcome: 'Hello ' + name + '!',
    };
    {
      person1: Person(username),
      person2: Person('Bob'),
    }"#;

        // .unwrap();
        unsafe {
            ext_string(
                &mut *vm, // name.as_ptr() as *const c_char,
                "name",   // value.as_ptr() as *const c_char,
                "afshan",
            );
        }

        let result = unsafe { jsonnet_evaluate_snippet(&mut *vm, filename, snippet) };
        println!("{}", result);
        // }
    }

    #[test]
    fn do_regex_test() {
        let hay = r"Question: Which magazine was started first Arthur's Magazine or First for Women?
        Thought 1: I need to search Arthur's Magazine and First for Women, and find which was
        started first.
        Action 1: Search[Arthur's Magazine]
        Observation 1: Arthur's Magazine (1844-1846) was an American literary periodical published
        in Philadelphia in the 19th century.
        Thought 2: Arthur's Magazine was started in 1844. I need to search First for Women
        next.
        Action 2: Search[First for Women]";
        // get words in Search[]
        let re = Regex::new(r"Observation 1: (.*)\.").unwrap();

        let mut search = Vec::new();
        for cap in re.captures_iter(hay) {
            search.push(cap[1].to_string());
        }

        println!("pattern found {:?}", search);
    }
}
