import Jsonnet from "../src/jsonnet.js";
let jsonnet = new Jsonnet();
import { CallBackClass } from "../src/jsonnet_wasm.js"
let jsfunc = new CallBackClass();

// simple example

// function hello(name) {
//     console.log("hello "+JSON.parse(name).name)
// }

// jsfunc.arg = JSON.stringify({name:"afshan"})

// jsfunc.call_native_js_func(hello)

// add function 

function add(args) {
    let parsed = JSON.parse(args)
    console.log(parsed)
    return parsed.a + parsed.b;
}

jsfunc.arg = JSON.stringify({a:2,b:3})

console.log(jsfunc.call_native_js_func(add))
