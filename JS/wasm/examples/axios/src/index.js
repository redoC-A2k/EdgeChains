import { Hono } from "hono";
const app = new Hono();
import { writeFileSync, STDIO } from "javy/fs"

app.get("/", (c) => {
    console.log("axios")
    return c.text("Hello World!");
});

function writeOutput(output) {
    const encodedOutput = new TextEncoder().encode(JSON.stringify(output));
    const buffer = new Uint8Array(encodedOutput);
    writeFileSync(STDIO.Stdout,buffer)
}

app.get("/axios", async (c) => {
    console.log("In axios")
    let result = await axios.get("https://jsonplaceholder.typicode.com/todos/1");
    let json = result.data;
    return c.json(json);
})

app.get("/fetch", async (c) => {
    try {
        let result = await fetch("https://jsonplaceholder.typicode.com/todos/1");
        let body = await result.json()
        return c.json(body);
    } catch (error) {
        console.log("Error occured - ")
        writeOutput(error);
        console.log(error);
        return c.json(error)
    }

})

globalThis._export = app;

// let result = await axios.get("https://jsonplaceholder.typicode.com/todos/1");
// let json = result.data;
// console.log(json);