import { Hono } from "hono";
import { readFileSync, writeFileSync, STDIO } from "javy/fs"
// import axios from "axios";
const app = new Hono();

function writeOutput(output) {
    const encodedOutput = new TextEncoder().encode(output);
    const buffer = new Uint8Array(encodedOutput);
    // Stdout file descriptor
    writeFileSync(STDIO.Stdout, buffer);
}

app.get("/hello", (c) => {
    return c.text("Hello World")
})

app.get('/post', async (c) => {
    try {
        let response = await fetch('https://jsonplaceholder.typicode.com/posts', {
            method: 'POST',
            body: JSON.stringify({
                title: 'foo',
                body: 'bar',
                userId: 1,
            }),
            headers: {
                'Content-type': 'application/json',
            },
        })
        let body = JSON.parse(response.body);
        response.body = body;
        return c.json(response);
    } catch (error) {
        console.log("error occured")
        writeOutput(error)
        let output = JSON.stringify(error);
        writeOutput(output)
        return c.json(output);
    }
})

app.get('/get', async (c) => {
    try {
        let response = await fetch('https://jsonplaceholder.typicode.com/todos/1', { method: "GET" });
        let body = JSON.parse(response.body);
        response.body = body;
        return c.json(response);
    } catch (error) {
        console.log("error occured")
        writeOutput(error)
        let output = JSON.stringify(error);
        writeOutput(output)
        return c.json(output);
    }
})

app.get('/axiosget', async (c) => {
    try {
        let repsonse = await axios.get('https://jsonplaceholder.typicode.com/todos/1');
        return c.json(repsonse);
    } catch (error) {
        console.log("error occured")
        writeOutput(error)
        return c.json(error);
    }
})

app.fire()