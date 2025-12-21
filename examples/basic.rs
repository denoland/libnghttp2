use libnghttp2::*;
use std::ffi::CStr;
use std::ptr;

fn main() {
  unsafe {
    let version_ptr = nghttp2_version(0);
    let version_str = CStr::from_ptr(version_ptr as *const i8);
    println!("nghttp2 version: {}", version_str.to_str().unwrap());

    let mut option: *mut nghttp2_option = ptr::null_mut();
    let ret = nghttp2_option_new(&mut option as *mut _);
    if ret != 0 {
      eprintln!("Failed to create options: {}", ret);
      return;
    }
    println!("Created session options successfully");

    nghttp2_option_set_no_auto_window_update(option, 1);
    nghttp2_option_set_peer_max_concurrent_streams(option, 100);
    println!("Configured session options");

    nghttp2_option_del(option);
    println!("Cleaned up options");

    let mut pri_spec: nghttp2_priority_spec = std::mem::zeroed();
    nghttp2_priority_spec_init(&mut pri_spec as *mut _, 0, 16, 0);
    println!(
      "Created priority spec: stream_id={}, weight={}, exclusive={}",
      pri_spec.stream_id, pri_spec.weight, pri_spec.exclusive
    );

    let error_msg = nghttp2_strerror(NGHTTP2_ERR_INVALID_ARGUMENT as i32);
    let error_str = CStr::from_ptr(error_msg as *const i8);
    println!("Example error message: {}", error_str.to_str().unwrap());

    println!("\nAll basic nghttp2 operations completed successfully!");
  }
}
