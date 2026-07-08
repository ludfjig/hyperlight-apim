root := justfile_directory()
policy_wasm := root / "wit" / "policy.wasm"

# List recipes.
default:
    @just --list

# Check dependencies and print install commands for any that are missing.
setup:
    #!/usr/bin/env bash
    set -uo pipefail
    miss=0
    check() {
        if ! command -v "$1" >/dev/null 2>&1; then
            echo "MISSING $1 -> install: $2"
            miss=1
        else
            echo "ok      $1"
        fi
    }
    check wasm-tools "cargo install wasm-tools"
    check cargo-component "cargo install cargo-component"
    check componentize-qjs "cargo install componentize-qjs-cli --locked"
    check hyperlight-wasm-aot "cargo install hyperlight-wasm-aot --version 0.14.0 --locked"
    check wasmtime "cargo install wasmtime-cli"
    check dotnet "https://dotnet.microsoft.com/download"
    check rustup "https://rustup.rs"

# Generate the binary component type from the WIT.
wit:
    wasm-tools component wit {{root}}/wit/policy.wit -w -o {{policy_wasm}}

# Build all guest components.
build-guests: build-auth build-pathblock

# Rust auth-check guest (customer A).
build-auth:
    cd {{root}}/guests/auth_check && cargo component build --release --target wasm32-unknown-unknown
    just _aot {{root}}/guests/auth_check/target/wasm32-unknown-unknown/release/auth_check.wasm {{root}}/guests/auth_check/auth_check.aot

# JS path-block guest (customer B).
build-pathblock:
    cd {{root}}/guests/path_block && componentize-qjs --wit {{root}}/wit --world policy --js policy.js --stub-wasi --sync --opt-size --minify -o path_block.wasm
    just _aot {{root}}/guests/path_block/path_block.wasm {{root}}/guests/path_block/path_block.aot

# AOT-compile a component. The tool version must match the hyperlight-wasm
# version policy_ffi links, so its wasmtime matches the guest runtime.
_aot wasm out:
    hyperlight-wasm-aot compile --component {{wasm}} {{out}}

# Verify each guest standalone with the wasmtime CLI.
verify-guests:
    #!/usr/bin/env bash
    set -e
    A={{root}}/guests/auth_check/target/wasm32-unknown-unknown/release/auth_check.wasm
    B={{root}}/guests/path_block/path_block.wasm
    echo "auth_check no-auth:"; wasmtime run --invoke 'on-request({method: "GET", path: "/x", headers: []})' "$A"
    echo "auth_check with-auth:"; wasmtime run --invoke 'on-request({method: "GET", path: "/x", headers: [{name: "authorization", value: "Bearer x"}]})' "$A"
    echo "path_block /products:"; wasmtime run --invoke 'on-request({method: "GET", path: "/products", headers: []})' "$B"
    echo "path_block /admin:"; wasmtime run --invoke 'on-request({method: "GET", path: "/admin/x", headers: []})' "$B"

# Build the policy_ffi cdylib (builds the specialized guest runtime, heavy).
build-ffi: wit
    cd {{root}}/policy_ffi && WIT_WORLD={{policy_wasm}} cargo build --release

# Build the C# gateway and wrapper.
build-gateway:
    cd {{root}}/dotnet && dotnet build Gateway/Gateway.csproj -c Release

# Build everything in order.
build: wit build-guests build-ffi build-gateway

# Start the gateway.
run: build-gateway
    cd {{root}}/dotnet/Gateway && POLICY_DEMO_ROOT={{root}} POLICY_FFI_LIB={{root}}/policy_ffi/target/release/libpolicy_ffi.so dotnet run -c Release

# Run the demo curl session against a running gateway.
demo:
    chmod +x {{root}}/demo.sh && {{root}}/demo.sh
