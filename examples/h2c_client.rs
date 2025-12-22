//! Example toy h2c client using libnghttp2.

use libnghttp2::*;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::{ptr, slice};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Response {
  status: u16,
  headers: Vec<(String, String)>,
  body: Vec<u8>,
}

impl Response {
  fn body_str(&self) -> Result<&str> {
    Ok(std::str::from_utf8(&self.body)?)
  }
}

struct Request {
  authority: String,
  path: String,
}

impl Request {
  fn builder() -> RequestBuilder {
    RequestBuilder::default()
  }
}

#[derive(Default)]
struct RequestBuilder {
  authority: Option<String>,
  path: Option<String>,
}

impl RequestBuilder {
  fn authority(mut self, authority: impl Into<String>) -> Self {
    self.authority = Some(authority.into());
    self
  }

  fn path(mut self, path: impl Into<String>) -> Self {
    self.path = Some(path.into());
    self
  }

  fn build(self) -> Result<Request> {
    Ok(Request {
      authority: self.authority.ok_or("authority is required")?,
      path: self.path.unwrap_or_else(|| "/".into()),
    })
  }
}

struct Session {
  ptr: *mut nghttp2_session,
  callbacks: *mut nghttp2_session_callbacks,
}

impl Session {
  fn new(user_data: *mut std::os::raw::c_void) -> Result<Self> {
    unsafe {
      let mut callbacks = ptr::null_mut();
      check(nghttp2_session_callbacks_new(&mut callbacks))?;

      nghttp2_session_callbacks_set_send_callback(callbacks, Some(send_cb));
      nghttp2_session_callbacks_set_on_frame_recv_callback(
        callbacks,
        Some(frame_recv_cb),
      );
      nghttp2_session_callbacks_set_on_data_chunk_recv_callback(
        callbacks,
        Some(data_cb),
      );
      nghttp2_session_callbacks_set_on_header_callback(
        callbacks,
        Some(header_cb),
      );
      nghttp2_session_callbacks_set_on_stream_close_callback(
        callbacks,
        Some(stream_close_cb),
      );

      let mut session_ptr = ptr::null_mut();
      check(nghttp2_session_client_new(
        &mut session_ptr,
        callbacks,
        user_data,
      ))?;

      check(nghttp2_submit_settings(
        session_ptr,
        NGHTTP2_FLAG_NONE as u8,
        ptr::null(),
        0,
      ))?;

      Ok(Self {
        ptr: session_ptr,
        callbacks,
      })
    }
  }

  fn submit(&mut self, req: &Request) -> Result<i32> {
    unsafe {
      let headers = [
        header(":method", "GET"),
        header(":scheme", "http"),
        header(":authority", &req.authority),
        header(":path", &req.path),
      ];

      let stream_id = nghttp2_submit_request(
        self.ptr,
        ptr::null(),
        headers.as_ptr(),
        headers.len(),
        ptr::null(),
        ptr::null_mut(),
      );

      check(stream_id)?;
      check(nghttp2_session_send(self.ptr))?;
      Ok(stream_id)
    }
  }

  fn wants_io(&self) -> bool {
    unsafe {
      nghttp2_session_want_read(self.ptr) != 0
        || nghttp2_session_want_write(self.ptr) != 0
    }
  }

  fn recv(&mut self, data: &[u8]) -> Result<()> {
    unsafe {
      check(
        nghttp2_session_mem_recv(self.ptr, data.as_ptr(), data.len()) as i32,
      )?;
      check(nghttp2_session_send(self.ptr))
    }
  }
}

impl Drop for Session {
  fn drop(&mut self) {
    unsafe {
      if !self.ptr.is_null() {
        nghttp2_session_del(self.ptr);
      }
      if !self.callbacks.is_null() {
        nghttp2_session_callbacks_del(self.callbacks);
      }
    }
  }
}

struct Context {
  stream: TcpStream,
  response: Response,
  stream_id: i32,
  done: bool,
}

struct Client {
  context: Box<Context>,
  session: Session,
}

impl Client {
  fn connect(host: &str) -> Result<Self> {
    let stream = TcpStream::connect(format!("{}:80", host))?;

    let mut context = Box::new(Context {
      stream,
      response: Response {
        status: 0,
        headers: Vec::new(),
        body: Vec::new(),
      },
      stream_id: -1,
      done: false,
    });

    let user_data = context.as_mut() as *mut Context as *mut _;
    let session = Session::new(user_data)?;

    Ok(Self { context, session })
  }

  fn execute(mut self, request: Request) -> Result<Response> {
    self.context.stream_id = self.session.submit(&request)?;

    let mut buf = [0u8; 16384];
    while !self.context.done && self.session.wants_io() {
      match self.context.stream.read(&mut buf) {
        Ok(0) => break,
        Ok(n) => self.session.recv(&buf[..n])?,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
        Err(e) => return Err(e.into()),
      }
    }

    Ok(std::mem::replace(
      &mut self.context.response,
      Response {
        status: 0,
        headers: Vec::new(),
        body: Vec::new(),
      },
    ))
  }
}

fn check(code: i32) -> Result<()> {
  if code < 0 {
    Err(format!("nghttp2 error: {}", code).into())
  } else {
    Ok(())
  }
}

fn header(name: &str, value: &str) -> nghttp2_nv {
  let name = name.as_bytes();
  let value = value.as_bytes();
  nghttp2_nv {
    name: name.as_ptr() as *mut u8,
    value: value.as_ptr() as *mut u8,
    namelen: name.len(),
    valuelen: value.len(),
    flags: NGHTTP2_NV_FLAG_NONE as u8,
  }
}

unsafe fn to_str(ptr: *const u8, len: usize) -> String {
  String::from_utf8_lossy(unsafe { slice::from_raw_parts(ptr, len) }).into()
}

unsafe extern "C" fn send_cb(
  _: *mut nghttp2_session,
  data: *const u8,
  len: usize,
  _: i32,
  user_data: *mut std::os::raw::c_void,
) -> isize {
  unsafe {
    let ctx = &mut *(user_data as *mut Context);
    match ctx.stream.write_all(slice::from_raw_parts(data, len)) {
      Ok(_) => len as isize,
      Err(_) => NGHTTP2_ERR_CALLBACK_FAILURE as isize,
    }
  }
}

unsafe extern "C" fn data_cb(
  _: *mut nghttp2_session,
  _: u8,
  stream_id: i32,
  data: *const u8,
  len: usize,
  user_data: *mut std::os::raw::c_void,
) -> i32 {
  unsafe {
    let ctx = &mut *(user_data as *mut Context);
    if stream_id == ctx.stream_id {
      ctx
        .response
        .body
        .extend_from_slice(slice::from_raw_parts(data, len));
    }
    0
  }
}

unsafe extern "C" fn frame_recv_cb(
  _: *mut nghttp2_session,
  frame: *const nghttp2_frame,
  user_data: *mut std::os::raw::c_void,
) -> i32 {
  unsafe {
    let ctx = &mut *(user_data as *mut Context);
    if (*frame).hd.stream_id == ctx.stream_id
      && ((*frame).hd.flags & NGHTTP2_FLAG_END_STREAM as u8) != 0
    {
      ctx.done = true;
    }
    0
  }
}

unsafe extern "C" fn header_cb(
  _: *mut nghttp2_session,
  frame: *const nghttp2_frame,
  name: *const u8,
  namelen: usize,
  value: *const u8,
  valuelen: usize,
  _: u8,
  user_data: *mut std::os::raw::c_void,
) -> i32 {
  unsafe {
    let ctx = &mut *(user_data as *mut Context);
    if (*frame).hd.stream_id == ctx.stream_id {
      let name_str = to_str(name, namelen);
      let value_str = to_str(value, valuelen);

      if name_str == ":status" {
        ctx.response.status = value_str.parse().unwrap_or(0);
      }
      ctx.response.headers.push((name_str, value_str));
    }
    0
  }
}

unsafe extern "C" fn stream_close_cb(
  _: *mut nghttp2_session,
  stream_id: i32,
  _: u32,
  user_data: *mut std::os::raw::c_void,
) -> i32 {
  unsafe {
    let ctx = &mut *(user_data as *mut Context);
    if stream_id == ctx.stream_id {
      ctx.done = true;
    }
    0
  }
}

fn main() -> Result<()> {
  let request = Request::builder()
    .authority("nghttp2.org")
    .path("/")
    .build()?;

  let client = Client::connect("nghttp2.org")?;
  let response = client.execute(request)?;

  println!("Status: {}", response.status);
  println!("\nHeaders:");
  for (name, value) in
    response.headers.iter().filter(|(n, _)| !n.starts_with(':'))
  {
    println!("  {}: {}", name, value);
  }

  println!("\nBody ({} bytes):", response.body.len());
  if let Ok(body) = response.body_str() {
    let preview = body.chars().take(500).collect::<String>();
    println!("{}{}", preview, if body.len() > 500 { "..." } else { "" });
  }

  Ok(())
}
