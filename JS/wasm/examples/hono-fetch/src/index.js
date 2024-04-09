import { Hono } from "hono";
import { readFileSync, writeFileSync, STDIO } from "javy/fs"
// import axios from "axios";
const app = new Hono();

function writeOutput(output) {
    // const encodedOutput = new TextEncoder().encode(JSON.stringify(output));
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
                'Content-type': 'application/json; charset=UTF-8',
            },
        })
        console.log(response);
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
        console.log(response);
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

// app.get('/axiostest', async (c)=>{
//     try {
//         let response = await axos.get('https://jsonplaceholder.typicode.com/todos/1');
//         console.log(response)
//         console.log(response.data)
//     } catch (error) {
//         console.log("error occured")
//         console.log(error)        
//     }
// })

app.fire()