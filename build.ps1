# Builds the WebAssembly module into pkg/, which index.html loads.
cargo build --release --target wasm32-unknown-unknown
if ($?) {
  wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/release/yapilacaklar.wasm
}
