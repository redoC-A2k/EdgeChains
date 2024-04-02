import Jsonnet from "../src/jsonnet.js";
let jsonnet = new Jsonnet();

function hello(name) {
    console.log("hello "+name)
}
globalThis.hello = hello;

jsonnet.javascriptNative("hello",globalThis,["afshan"])
// jsonnet.javascriptNativeEval("hello",[])