# WebGPU Testing

# Stack:
* WebGPU
* Skia
* Imgui
* Javascript/Wasm
* FFMPEG

# Supported Toolchain
 * Typescript
 * C#
 * Rust
 * WebAssembly

# Next: SVG as the source of truth 
We must be able to use XML/SVG as the source of truth for the elements

We will then need a way to map the control rig to it, declaratively

Make parts appear and disappear based on control rig parameters.

Still, JS scripting will be useful for more dynamic stuff like mane effects.

Or more complex control rigs like the eyes.

SO:

Find a away to turn 

SVG -> INTERNAL OP LIST -> SKIA COMMAND LIST

Where Internal Op List is a data structure we can modify at runtime to render different things.

There are probably no squares on the Internal Op List, just path ops.

Then the data flows is as follows:

Control Rig -> PonySolver -> ViewSelector -> Render

Control Rig: is the 3D thing the user is manipulating

PonySolver: a component that given the camera view position, and 
shape database, return a set of datastructures the view selector
can use to solve the control rig for the current view target

ViewSelector: a component that morphs and selects path ops
based on typed input parameters


Render: Just a translator of PathOps to Skia draw commands



This will be the easy part, the hard part will be building the shape and motion database.

The face control rig will probably take 1/4 of that tine.


``` 
cargo build --profile profiling && ./target/profiling/pony-renderer
``` 