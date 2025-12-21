use libnghttp2::*;
use std::ffi::CStr;
use std::ptr;

#[test]
fn test_version() {
  unsafe {
    let version = nghttp2_version(0);
    assert!(!version.is_null());

    let version_str = CStr::from_ptr(version as *const i8);
    let version_str = version_str.to_str().unwrap();
    assert!(!version_str.is_empty());
    println!("nghttp2 version: {}", version_str);
  }
}

#[test]
fn test_strerror() {
  unsafe {
    let error_str = nghttp2_strerror(NGHTTP2_ERR_INVALID_ARGUMENT as i32);
    assert!(!error_str.is_null());

    let error_str = CStr::from_ptr(error_str as *const i8);
    let error_str = error_str.to_str().unwrap();
    assert!(!error_str.is_empty());
    println!("Error string: {}", error_str);
  }
}

#[test]
fn test_session_callbacks_new_del() {
  unsafe {
    let mut callbacks: *mut nghttp2_session_callbacks = ptr::null_mut();
    let ret = nghttp2_session_callbacks_new(&mut callbacks as *mut _);
    assert_eq!(ret, 0);
    assert!(!callbacks.is_null());

    nghttp2_session_callbacks_del(callbacks);
  }
}

#[test]
fn test_session_client_new_del() {
  unsafe {
    let mut callbacks: *mut nghttp2_session_callbacks = ptr::null_mut();
    let ret = nghttp2_session_callbacks_new(&mut callbacks as *mut _);
    assert_eq!(ret, 0);

    let mut session: *mut nghttp2_session = ptr::null_mut();
    let ret = nghttp2_session_client_new(
      &mut session as *mut _,
      callbacks,
      ptr::null_mut(),
    );
    assert_eq!(ret, 0);
    assert!(!session.is_null());

    nghttp2_session_del(session);
    nghttp2_session_callbacks_del(callbacks);
  }
}

#[test]
fn test_session_server_new_del() {
  unsafe {
    let mut callbacks: *mut nghttp2_session_callbacks = ptr::null_mut();
    let ret = nghttp2_session_callbacks_new(&mut callbacks as *mut _);
    assert_eq!(ret, 0);

    let mut session: *mut nghttp2_session = ptr::null_mut();
    let ret = nghttp2_session_server_new(
      &mut session as *mut _,
      callbacks,
      ptr::null_mut(),
    );
    assert_eq!(ret, 0);
    assert!(!session.is_null());

    nghttp2_session_del(session);
    nghttp2_session_callbacks_del(callbacks);
  }
}

#[test]
fn test_option_new_del() {
  unsafe {
    let mut option: *mut nghttp2_option = ptr::null_mut();
    let ret = nghttp2_option_new(&mut option as *mut _);
    assert_eq!(ret, 0);
    assert!(!option.is_null());

    nghttp2_option_del(option);
  }
}

#[test]
fn test_option_set_no_auto_window_update() {
  unsafe {
    let mut option: *mut nghttp2_option = ptr::null_mut();
    let ret = nghttp2_option_new(&mut option as *mut _);
    assert_eq!(ret, 0);

    nghttp2_option_set_no_auto_window_update(option, 1);
    nghttp2_option_del(option);
  }
}

#[test]
fn test_option_set_peer_max_concurrent_streams() {
  unsafe {
    let mut option: *mut nghttp2_option = ptr::null_mut();
    let ret = nghttp2_option_new(&mut option as *mut _);
    assert_eq!(ret, 0);

    nghttp2_option_set_peer_max_concurrent_streams(option, 100);
    nghttp2_option_del(option);
  }
}

#[test]
fn test_priority_spec_init() {
  unsafe {
    let mut pri_spec: nghttp2_priority_spec = std::mem::zeroed();
    nghttp2_priority_spec_init(&mut pri_spec as *mut _, 0, 16, 0);

    assert_eq!(pri_spec.stream_id, 0);
    assert_eq!(pri_spec.weight, 16);
    assert_eq!(pri_spec.exclusive, 0);
  }
}

#[test]
fn test_priority_spec_default_init() {
  unsafe {
    let mut pri_spec: nghttp2_priority_spec = std::mem::zeroed();
    nghttp2_priority_spec_default_init(&mut pri_spec as *mut _);

    // Default priority spec has stream_id 0, weight 16, and not exclusive
    assert_eq!(pri_spec.stream_id, 0);
    assert_eq!(pri_spec.weight, NGHTTP2_DEFAULT_WEIGHT as i32);
    assert_eq!(pri_spec.exclusive, 0);
  }
}

#[test]
fn test_priority_spec_check_default() {
  unsafe {
    let mut pri_spec: nghttp2_priority_spec = std::mem::zeroed();
    nghttp2_priority_spec_default_init(&mut pri_spec as *mut _);

    let result = nghttp2_priority_spec_check_default(&pri_spec as *const _);
    assert_ne!(result, 0);

    // Modify and check it's no longer default
    pri_spec.stream_id = 1;
    let result = nghttp2_priority_spec_check_default(&pri_spec as *const _);
    assert_eq!(result, 0);
  }
}

#[test]
fn test_submit_settings() {
  unsafe {
    let mut callbacks: *mut nghttp2_session_callbacks = ptr::null_mut();
    let ret = nghttp2_session_callbacks_new(&mut callbacks as *mut _);
    assert_eq!(ret, 0);

    let mut session: *mut nghttp2_session = ptr::null_mut();
    let ret = nghttp2_session_client_new(
      &mut session as *mut _,
      callbacks,
      ptr::null_mut(),
    );
    assert_eq!(ret, 0);

    let settings = [
      nghttp2_settings_entry {
        settings_id: NGHTTP2_SETTINGS_MAX_CONCURRENT_STREAMS as i32,
        value: 100,
      },
      nghttp2_settings_entry {
        settings_id: NGHTTP2_SETTINGS_INITIAL_WINDOW_SIZE as i32,
        value: 65535,
      },
    ];

    let ret = nghttp2_submit_settings(
      session,
      NGHTTP2_FLAG_NONE as u8,
      settings.as_ptr(),
      settings.len(),
    );
    assert_eq!(ret, 0);

    nghttp2_session_del(session);
    nghttp2_session_callbacks_del(callbacks);
  }
}

#[test]
fn test_session_want_read_write() {
  unsafe {
    let mut callbacks: *mut nghttp2_session_callbacks = ptr::null_mut();
    let ret = nghttp2_session_callbacks_new(&mut callbacks as *mut _);
    assert_eq!(ret, 0);

    let mut session: *mut nghttp2_session = ptr::null_mut();
    let ret = nghttp2_session_client_new(
      &mut session as *mut _,
      callbacks,
      ptr::null_mut(),
    );
    assert_eq!(ret, 0);

    // Submit settings to make session want to write
    let settings = [nghttp2_settings_entry {
      settings_id: NGHTTP2_SETTINGS_MAX_CONCURRENT_STREAMS as i32,
      value: 100,
    }];

    let ret = nghttp2_submit_settings(
      session,
      NGHTTP2_FLAG_NONE as u8,
      settings.as_ptr(),
      settings.len(),
    );
    assert_eq!(ret, 0);

    // After submitting settings, session should want to write
    let want_write = nghttp2_session_want_write(session);
    assert_ne!(want_write, 0);

    // Should still want to read (waiting for responses)
    let want_read = nghttp2_session_want_read(session);
    assert_ne!(want_read, 0);

    nghttp2_session_del(session);
    nghttp2_session_callbacks_del(callbacks);
  }
}

#[test]
fn test_hd_deflate_inflate() {
  unsafe {
    let mut deflater: *mut nghttp2_hd_deflater = ptr::null_mut();
    let ret = nghttp2_hd_deflate_new(&mut deflater as *mut _, 4096);
    assert_eq!(ret, 0);
    assert!(!deflater.is_null());

    let mut inflater: *mut nghttp2_hd_inflater = ptr::null_mut();
    let ret = nghttp2_hd_inflate_new(&mut inflater as *mut _);
    assert_eq!(ret, 0);
    assert!(!inflater.is_null());

    nghttp2_hd_deflate_del(deflater);
    nghttp2_hd_inflate_del(inflater);
  }
}

#[test]
fn test_check_header_name() {
  unsafe {
    let valid = b"content-type";
    let ret = nghttp2_check_header_name(valid.as_ptr(), valid.len());
    assert_ne!(ret, 0);

    let invalid = b"Content-Type";
    let ret = nghttp2_check_header_name(invalid.as_ptr(), invalid.len());
    assert_eq!(ret, 0);
  }
}

#[test]
fn test_check_header_value() {
  unsafe {
    let valid = b"application/json";
    let ret = nghttp2_check_header_value(valid.as_ptr(), valid.len());
    assert_ne!(ret, 0);

    let invalid = b"application\0json";
    let ret = nghttp2_check_header_value(invalid.as_ptr(), invalid.len());
    assert_eq!(ret, 0);
  }
}
