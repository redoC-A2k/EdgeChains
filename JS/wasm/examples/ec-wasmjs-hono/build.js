// import { build } from "esbuild";

// let runtime = process.argv[2];

// build({
//     entryPoints: ["src/index.js"],
//     bundle: true,
//     minify: true,
//     outfile: "bin/app.js",
//     format: "esm",
//     target: "esnext",
//     platform: "node",
//     // external: ["arakoo"],
//     define: {
//         "process.env.arakoo": JSON.stringify(runtime === "arakoo"),
//     },
// }).catch((error) => {
//     console.error(error);
//     process.exit(1);
// });

import { build } from "esbuild"
// import DynamicImport from "@rtvision/esbuild-dynamic-import"
let runtime = process.argv[2];
import fs from "fs"

async function compile() {
    try {
        let buildConfig = {
            entryPoints: ["src/index.js"],
            bundle: true,
            // minify: true,
            minifySyntax: true,
            outfile: "out.js",
            format: "esm",
            write: false,
            target: "esnext",
            platform: "node",
            // inject:["evaluateShim.js"],
            define: {
                "process.env.arakoo": JSON.stringify(runtime === "arakoo"),
            },
        }
        let result = await build(buildConfig)

        // replace .evaluateFile(path) with .evaluateFile(import(path))
        let output = result.outputFiles[0].text;
        let pattern = /\.[\s]*evaluateFile[\s]*(\(.*\))/g;

        let matches = output.match(pattern)
        for (let match of matches) {
            // console.log(match)
            let argument = match.match(/\(.*\)/g)
            // console.log(argument)
            output = output.replace(match, `.evaluateFile(require${argument})`)
        }

        fs.writeFileSync("out.js", output)

        await build({
            entryPoints: ["out.js"],
            bundle: true,
            // minify: true,
            minifySyntax: true,
            outfile: "dist.js",
            format: "esm",
            target: "esnext",
            platform: "node",
            // plugins:[jsonnetLoadPlugin]
            loader: {
                ".jsonnet": "text",
            },
        })

    } catch (error) {
        console.log("error building index.js")
        console.error(error);
        process.exit(1);
    }
}

compile()