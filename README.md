# libnghttp2

[![Crates.io](https://img.shields.io/crates/v/libnghttp2.svg)](https://crates.io/crates/libnghttp2)

[Documentation](https://docs.rs/libnghttp2) | [C API](https://nghttp2.org/documentation/apiref.html)

_libnghttp2_ provides FFI bindings to the HTTP/2 framing layer of nghttp2 C library.

You can use it for raw frame parsing to implement rfc9113-compliant HTTP/2 clients, servers and proxies.

```rust
use libnghttp2::*;

let mut callbacks = std::ptr::null_mut();
nghttp2_session_callbacks_new(&mut callbacks);
nghttp2_session_callbacks_set_send_callback(callbacks, Some(send_cb));
nghttp2_session_callbacks_set_on_frame_recv_callback(callbacks, Some(frame_recv_cb));
nghttp2_session_callbacks_set_on_data_chunk_recv_callback(callbacks, Some(data_cb));

let mut session_ptr = ptr::null_mut();
nghttp2_session_client_new(&mut session_ptr, callbacks, user_data);

// Submit HTTP/2 request
let headers = [
  nghttp2_nv { name: b":method\0".as_ptr() as *mut u8, value: b"GET\0".as_ptr() as *mut u8, /* ... */ },
  nghttp2_nv { name: b":scheme\0".as_ptr() as *mut u8, value: b"http\0".as_ptr() as *mut u8, /* ... */ },
  // ...
];
nghttp2_submit_request(session_ptr, ptr::null(), headers.as_ptr(), headers.len(), ptr::null(), ptr::null_mut());
```

See [examples/h2c_client.rs](examples/h2c_client.rs) for a simple HTTP/2 client implementation.

## Linking

By default this crate builds and statically links the bundled copy of nghttp2,
so it works out of the box with no system dependencies.

Packagers (e.g. distros) can instead link against a system libnghttp2 discovered
via `pkg-config`. This is opt-in and falls back to the bundled build if no
suitable system library is found:

- Enable the `system` Cargo feature:

  ```toml
  [dependencies]
  libnghttp2 = { version = "1", features = ["system"] }
  ```

- Or set the `LIBNGHTTP2_SYS_USE_PKG_CONFIG=1` environment variable at build
  time (no source changes required):

  ```sh
  LIBNGHTTP2_SYS_USE_PKG_CONFIG=1 cargo build
  ```

The crate declares `links = "nghttp2"`, so downstream `-sys`-style consumers can
read `DEP_NGHTTP2_ROOT` and `DEP_NGHTTP2_INCLUDE` from their build scripts.
