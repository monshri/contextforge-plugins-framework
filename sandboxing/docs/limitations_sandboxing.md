## Limitations of Wasm Sandboxing

`create-file: func(filename: string, content: string) -> string;`
This is a synchronous function signature in the WebAssembly Component Model. The WIT specification currently doesn't have native support for async functions with async/await semantics.