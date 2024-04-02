use std::path::Path;

use jrsonnet_evaluator::{
    apply_tla,
    function::{builtin, TlaArg},
    gc::GcHashMap,
    manifest::{JsonFormat, ManifestFormat},
    tb,
    trace::{CompactFormat, PathResolver, TraceFormat},
    FileImportResolver, ObjValueBuilder, State, Thunk, Val,
};
use jrsonnet_parser::IStr;
use wasm_bindgen::prelude::*;

mod context;

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

#[wasm_bindgen]
pub fn jsonnet_make() -> *mut VM {
    let state = State::default();
    state.settings_mut().import_resolver = tb!(FileImportResolver::default());
    // state.set_context_initializer((jrsonnet_stdlib::ContextInitializer::new(
    //     state.clone(),
    //     PathResolver::new_cwd_fallback(),
    // ),ArakooContext::ArakooContextInitializer::default()));
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
    let context_initializer = &*vm.state.context_initializer();
    println!("{:?}", context_initializer.as_any().type_id());

    let context_initializer = vm.state.context_initializer();

    println!(
        "Type of context initializer: {:?}",
        std::any::type_name_of_val(&*context_initializer)
    );

    context_initializer
        .as_any()
        .downcast_ref::<context::ArakooContextInitializer>()
        .expect("only stdlib context initializer supported")
        .add_ext_var(key.into(), Val::Str(value.into()));
}

#[wasm_bindgen]
pub struct CallBackClass {
    arg: String,
}

#[wasm_bindgen]
impl CallBackClass {
    #[wasm_bindgen(constructor)]
    pub extern "C" fn new() -> CallBackClass {
        CallBackClass {
            arg: String::from(""),
        }
    }

    pub extern "C" fn call_native_js_func(&self, f: &js_sys::Function) -> Result<JsValue, JsValue> {
        let this = JsValue::null();
        // for x in self.args {
        // let x = JsValue::from(x);
        let result = f.call1(&this, &JsValue::from_str(&self.arg));
        println!("Result of calling JS function: {:?}", result);
        return result;
        // }
    }

    #[wasm_bindgen(getter)]
    pub extern "C" fn arg(&self) -> String {
        self.arg.clone()
    }

    #[wasm_bindgen(setter)]
    pub extern "C" fn set_arg(&mut self, arg: String) {
        self.arg = arg;
    }
}

// Define a Rust function that accepts a JavaScript callback function
#[wasm_bindgen]
pub fn execute_callback(callback: js_sys::Function) {
    // Call the JavaScript callback function within the closure
    callback.call0(&JsValue::undefined()).unwrap();

    // Ensure the closure is deallocated to prevent memory leaks
    // closure.forget();
}

// Function to register the native callback
// pub extern "C" fn register_native_callback(vm: *mut VM, name: &str, ctx: &NativeContext) {
//     let vm = unsafe { &*vm };
//     let any_resolver = vm.state.context_initializer();
//     any_resolver
//         .as_any()
//         .downcast_ref::<jrsonnet_stdlib::ContextInitializer>()
//         .expect("only stdlib context initializer supported")
//         .add_native(
//             name,
//             NativeCallback::new(vec!["a".to_string(), "b".to_string()], NativeAddCallback),
//         );
// }

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
