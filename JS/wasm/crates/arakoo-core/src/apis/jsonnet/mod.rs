use super::{wit::edgechains, APIConfig, JSApiSet};
use arakoo_jsonnet;
use javy::quickjs::{JSContextRef, JSValue, JSValueRef};

pub(super) struct Jsonnet;

impl JSApiSet for Jsonnet {
    fn register(&self, runtime: &javy::Runtime, _config: &APIConfig) -> anyhow::Result<()> {
        let context = runtime.context();
        let global = context.global_object()?;

        global.set_property(
            "__jsonnet_make",
            context.wrap_callback(jsonnet_make_closure())?,
        )?;
        global.set_property(
            "__jsonnet_ext_string",
            context.wrap_callback(jsonnet_ext_string_closure())?,
        )?;
        global.set_property(
            "__jsonnet_evaluate_snippet",
            context.wrap_callback(jsonnet_evaluate_snippet_closure())?,
        )?;
        global.set_property(
            "__jsonnet_evaluate_file",
            context.wrap_callback(jsonnet_evaluate_file_closure())?,
        )?;
        global.set_property(
            "__jsonnet_destroy",
            context.wrap_callback(jsonnet_destroy_closure())?,
        )?;
        Ok(())
    }
}

fn jsonnet_make_closure(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx, _this, args| {
        let ptr = arakoo_jsonnet::jsonnet_make();
        Ok(JSValue::from(ptr as u64 as f64))
    }
}

fn jsonnet_ext_string_closure(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx, _this, args| {
        // check the number of arguments
        if args.len() != 3 {
            return Err(anyhow::anyhow!(
                "Expected 2 arguments, got {}",
                args.len() - 1
            ));
        }
        let vm = args.get(0).unwrap().as_f64()?;
        let key = args.get(1).unwrap().to_string();
        let value = args.get(2).unwrap().to_string();
        // edgechains::jsonnet::jsonnet_ext_string(vm as u64, &key, &value);
        let ptr = vm as u64;
        arakoo_jsonnet::jsonnet_ext_string(ptr as *mut arakoo_jsonnet::VM, &key, &value);
        Ok(JSValue::Undefined)
    }
}

fn jsonnet_evaluate_snippet_closure(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx, _this, args| {
        // check the number of arguments
        if args.len() != 2 {
            return Err(anyhow::anyhow!("Expected 2 arguments, got {}", args.len()));
        }
        let vm = args.get(0).unwrap().as_f64()?;
        let code = args.get(1).unwrap().to_string();
        let code = code.as_str();
        // let out = edgechains::jsonnet::jsonnet_evaluate_snippet(vm as u64, "snippet", code);
        let out = arakoo_jsonnet::jsonnet_evaluate_snippet(
            vm as u64 as *mut arakoo_jsonnet::VM,
            "snippet",
            code,
        );
        Ok(out.into())
    }
}

fn jsonnet_evaluate_file_closure(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx, _this, args| {
        // check the number of arguments
        if args.len() != 2 {
            return Err(anyhow::anyhow!("Expected 2 arguments, got {}", args.len()));
        }
        let vm = args.get(0).unwrap().as_f64()?;
        let path = args.get(1).unwrap().to_string();
        let code = edgechains::utils::read_file(path.as_str());
        let out = arakoo_jsonnet::jsonnet_evaluate_snippet(
            vm as u64 as *mut arakoo_jsonnet::VM,
            "snippet",
            &code,
        );
        Ok(out.into())
    }
}

fn jsonnet_destroy_closure(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx, _this, args| {
        // check the number of arguments
        if args.len() != 1 {
            return Err(anyhow::anyhow!("Expected 1 arguments, got {}", args.len()));
        }
        let vm = args.get(0).unwrap().as_f64()?;
        arakoo_jsonnet::jsonnet_destroy(vm as u64 as *mut arakoo_jsonnet::VM);
        Ok(JSValue::Undefined)
    }
}
