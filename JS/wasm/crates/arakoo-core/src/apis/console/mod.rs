use std::io::Write;

use anyhow::Result;
use javy::{
    quickjs::{JSContextRef, JSValue, JSValueRef},
    Runtime,
};
use quickjs_wasm_rs::{from_qjs_value, Exception};

use super::{APIConfig, JSApiSet};

pub(super) use config::ConsoleConfig;
pub use config::LogStream;

mod config;

pub(super) struct Console {}

impl Console {
    pub(super) fn new() -> Self {
        Console {}
    }
}

impl JSApiSet for Console {
    fn register(&self, runtime: &Runtime, config: &APIConfig) -> Result<()> {
        register_console(
            runtime.context(),
            config.console.log_stream.to_stream(),
            config.console.error_stream.to_stream(),
        )
    }
}

fn register_console<T, U>(context: &JSContextRef, log_stream: T, error_stream: U) -> Result<()>
where
    T: Write + 'static,
    U: Write + 'static,
{
    let console_log_callback = context.wrap_callback(console_log_to(log_stream))?;
    let console_error_callback = context.wrap_callback(console_log_to(error_stream))?;
    let console_object = context.object_value()?;
    console_object.set_property("log", console_log_callback)?;
    console_object.set_property("error", console_error_callback)?;
    context
        .global_object()?
        .set_property("console", console_object)?;
    Ok(())
}

fn console_log_to<T>(
    mut stream: T,
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> Result<JSValue>
where
    T: Write + 'static,
{
    move |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
        // Write full string to in-memory destination before writing to stream since each write call to the stream
        // will invoke a hostcall.
        let mut log_line = String::new();
        for (i, arg) in args.iter().enumerate() {
            if arg.is_exception() {
                let exception =
                    Exception::from(*arg).expect("Unable to convert jsvaluleref to exception");
                let err = exception.into_error();
                log_line = format!("Exception : {:?}", err);
            } else {
                let val: JSValue = from_qjs_value(*arg)?;
                if i != 0 {
                    log_line.push(' ');
                }
                if !arg.is_undefined() {
                    let proto = arg.get_property("__proto__").unwrap().to_string();
                    if proto.contains("rror") {
                        log_line.push_str(&format!(
                            "__proto__ is {} Error in js evaluation : {:?}",
                            proto, val
                        ));
                    } else {
                        let line: String = log_js_value(&val);
                        log_line.push_str(&line);
                    }
                } else {
                    let line: String = log_js_value(&val);
                    log_line.push_str(&line);
                }
            }
        }

        writeln!(stream, "{log_line}")?;

        Ok(JSValue::Undefined)
    }
}

fn log_js_value(arg: &JSValue) -> String {
    match arg {
        JSValue::String(s) => s.to_string(),
        JSValue::Int(n) => n.to_string(),
        JSValue::Bool(b) => b.to_string(),
        JSValue::Object(o) => {
            let flatten_obj = o
                .iter()
                .map(|(k, v)| format!("{}: {}", k, log_js_value(v)))
                .collect::<Vec<String>>()
                .join(", ");
            format!("Object = {{ {:?} }}", flatten_obj)
        }
        JSValue::Null => "null".to_string(),
        JSValue::Undefined => "undefined".to_string(),
        JSValue::Float(f) => f.to_string(),
        JSValue::Array(arr) => {
            let flatten_vec = arr
                .iter()
                .map(|v| log_js_value(v))
                .collect::<Vec<String>>()
                .join(", ");
            format!("Array = [{:?}]", flatten_vec)
        }
        JSValue::ArrayBuffer(buff) => {
            let buffer = buff
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            format!("ArrayBuffer = [{:?}]", buffer)
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use javy::Runtime;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::{cmp, io};

    use crate::apis::console::register_console;

    use super::{APIConfig, JSApiSet};

    use super::Console;

    #[test]
    fn test_register() -> Result<()> {
        let runtime = Runtime::default();
        Console::new().register(&runtime, &APIConfig::default())?;
        let console = runtime.context().global_object()?.get_property("console")?;
        assert!(console.get_property("log").is_ok());
        assert!(console.get_property("error").is_ok());
        Ok(())
    }

    #[test]
    fn test_console_log() -> Result<()> {
        let mut stream = SharedStream::default();

        let runtime = Runtime::default();
        let ctx = runtime.context();
        register_console(ctx, stream.clone(), stream.clone())?;

        ctx.eval_global("main", "console.log(\"hello world\");")?;
        assert_eq!(b"hello world\n", stream.buffer.borrow().as_slice());

        stream.clear();

        ctx.eval_global("main", "console.log(\"bonjour\", \"le\", \"monde\")")?;
        assert_eq!(b"bonjour le monde\n", stream.buffer.borrow().as_slice());

        stream.clear();

        ctx.eval_global(
            "main",
            "console.log(2.3, true, { foo: 'bar' }, null, undefined)",
        )?;
        assert_eq!(
            b"2.3 true [object Object] null undefined\n",
            stream.buffer.borrow().as_slice()
        );
        Ok(())
    }

    #[test]
    fn test_console_error() -> Result<()> {
        let mut stream = SharedStream::default();

        let runtime = Runtime::default();
        let ctx = runtime.context();
        register_console(ctx, stream.clone(), stream.clone())?;

        ctx.eval_global("main", "console.error(\"hello world\");")?;
        assert_eq!(b"hello world\n", stream.buffer.borrow().as_slice());

        stream.clear();

        ctx.eval_global("main", "console.error(\"bonjour\", \"le\", \"monde\")")?;
        assert_eq!(b"bonjour le monde\n", stream.buffer.borrow().as_slice());

        stream.clear();

        ctx.eval_global(
            "main",
            "console.error(2.3, true, { foo: 'bar' }, null, undefined)",
        )?;
        assert_eq!(
            b"2.3 true [object Object] null undefined\n",
            stream.buffer.borrow().as_slice()
        );
        Ok(())
    }

    #[derive(Clone)]
    struct SharedStream {
        buffer: Rc<RefCell<Vec<u8>>>,
        capacity: usize,
    }

    impl Default for SharedStream {
        fn default() -> Self {
            Self {
                buffer: Default::default(),
                capacity: usize::MAX,
            }
        }
    }

    impl SharedStream {
        fn clear(&mut self) {
            (*self.buffer).borrow_mut().clear();
        }
    }

    impl io::Write for SharedStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let available_capacity = self.capacity - (*self.buffer).borrow().len();
            let leftover = cmp::min(available_capacity, buf.len());
            (*self.buffer).borrow_mut().write(&buf[..leftover])
        }

        fn flush(&mut self) -> io::Result<()> {
            (*self.buffer).borrow_mut().flush()
        }
    }
}
