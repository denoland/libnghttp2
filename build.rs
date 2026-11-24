use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SOURCES: &[&str] = &[
  "nghttp2/lib/sfparse.c",
  "nghttp2/lib/nghttp2_alpn.c",
  "nghttp2/lib/nghttp2_buf.c",
  "nghttp2/lib/nghttp2_callbacks.c",
  "nghttp2/lib/nghttp2_debug.c",
  "nghttp2/lib/nghttp2_extpri.c",
  "nghttp2/lib/nghttp2_frame.c",
  "nghttp2/lib/nghttp2_hd.c",
  "nghttp2/lib/nghttp2_hd_huffman.c",
  "nghttp2/lib/nghttp2_hd_huffman_data.c",
  "nghttp2/lib/nghttp2_helper.c",
  "nghttp2/lib/nghttp2_http.c",
  "nghttp2/lib/nghttp2_map.c",
  "nghttp2/lib/nghttp2_mem.c",
  "nghttp2/lib/nghttp2_option.c",
  "nghttp2/lib/nghttp2_outbound_item.c",
  "nghttp2/lib/nghttp2_pq.c",
  "nghttp2/lib/nghttp2_priority_spec.c",
  "nghttp2/lib/nghttp2_queue.c",
  "nghttp2/lib/nghttp2_rcbuf.c",
  "nghttp2/lib/nghttp2_session.c",
  "nghttp2/lib/nghttp2_stream.c",
  "nghttp2/lib/nghttp2_submit.c",
  "nghttp2/lib/nghttp2_version.c",
  "nghttp2/lib/nghttp2_ratelim.c",
  "nghttp2/lib/nghttp2_time.c",
];

const HEADERS: &[&str] = &[
  "nghttp2/lib/sfparse.h",
  "nghttp2/lib/nghttp2_alpn.h",
  "nghttp2/lib/nghttp2_buf.h",
  "nghttp2/lib/nghttp2_callbacks.h",
  "nghttp2/lib/nghttp2_debug.h",
  "nghttp2/lib/nghttp2_extpri.h",
  "nghttp2/lib/nghttp2_frame.h",
  "nghttp2/lib/nghttp2_hd.h",
  "nghttp2/lib/nghttp2_hd_huffman.h",
  "nghttp2/lib/nghttp2_helper.h",
  "nghttp2/lib/nghttp2_http.h",
  "nghttp2/lib/nghttp2_int.h",
  "nghttp2/lib/nghttp2_map.h",
  "nghttp2/lib/nghttp2_mem.h",
  "nghttp2/lib/nghttp2_net.h",
  "nghttp2/lib/nghttp2_option.h",
  "nghttp2/lib/nghttp2_outbound_item.h",
  "nghttp2/lib/nghttp2_pq.h",
  "nghttp2/lib/nghttp2_priority_spec.h",
  "nghttp2/lib/nghttp2_queue.h",
  "nghttp2/lib/nghttp2_ratelim.h",
  "nghttp2/lib/nghttp2_rcbuf.h",
  "nghttp2/lib/nghttp2_session.h",
  "nghttp2/lib/nghttp2_stream.h",
  "nghttp2/lib/nghttp2_submit.h",
  "nghttp2/lib/nghttp2_time.h",
];

fn main() {
  let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
  let target = env::var("TARGET").expect("TARGET not set");

  // Rebuild when the system-linking opt-in changes.
  println!("cargo:rerun-if-env-changed=LIBNGHTTP2_SYS_USE_PKG_CONFIG");

  // Optionally link against a system libnghttp2 instead of the bundled copy.
  // Falls back to the bundled build when no suitable system library is found.
  if try_system_nghttp2(&out_dir) {
    return;
  }

  build_bundled(&out_dir, &target);
}

// Build and link the bundled nghttp2 sources (the default / fallback path).
fn build_bundled(out_dir: &Path, target: &str) {
  let nghttp2_version = parse_nghttp2_version();

  let install_dir = out_dir.join("i");
  let include_dir = install_dir.join("include");
  let lib_dir = install_dir.join("lib");

  setup_directories(&include_dir, &lib_dir);
  generate_version_header(&include_dir, &nghttp2_version);
  copy_main_header(&include_dir);

  build_nghttp2(target, &include_dir, &lib_dir);

  generate_pkgconfig(&install_dir, &include_dir, &lib_dir, &nghttp2_version);
  generate_bindings(out_dir, &include_dir);

  println!("cargo:root={}", install_dir.display());
  // Expose the bundled headers to downstream build scripts via
  // DEP_NGHTTP2_INCLUDE (enabled by `links = "nghttp2"`).
  println!("cargo:include={}", include_dir.display());

  // Emit rerun-if-changed directives to avoid unnecessary rebuilds
  emit_rerun_if_changed();
}

// Returns true if the requested system libnghttp2 was found and linked.
//
// System linking is opt-in: it is attempted only when the `system` Cargo
// feature is enabled or the LIBNGHTTP2_SYS_USE_PKG_CONFIG environment variable
// is set to a truthy value.
//
// The environment variable means "the system library is required": if it is set
// and no suitable library is found, the build fails instead of silently bundling
// nghttp2, which is what packagers (e.g. Linux distributions) need to rule out.
// The feature is opportunistic: when it is enabled but no suitable library is
// found, we return false so the caller falls back to the bundled build.
fn try_system_nghttp2(out_dir: &Path) -> bool {
  let required = system_linking_required();
  if !required && !system_linking_requested_by_feature() {
    return false;
  }

  let mut cfg = pkg_config::Config::new();
  // libnghttp2 in this crate tracks the 1.x series; require at least 1.0.0.
  cfg.atleast_version("1.0.0");
  cfg.print_system_libs(false);

  match cfg.probe("libnghttp2") {
    Ok(lib) => {
      // pkg-config has already emitted the rustc-link-lib / link-search
      // directives. Generate bindings against the system headers and expose
      // the include paths to downstream crates.
      generate_bindings_system(out_dir, &lib.include_paths);

      for path in &lib.include_paths {
        if let Some(path) = path.to_str() {
          println!("cargo:include={}", path);
        }
      }
      println!(
        "cargo:warning=libnghttp2: linking against system library (pkg-config)"
      );
      true
    }
    Err(err) if required => {
      panic!(
        "libnghttp2: LIBNGHTTP2_SYS_USE_PKG_CONFIG is set, but a suitable \
         system library was not found ({err}).\n\
         Install the libnghttp2 development files (and pkg-config), or unset \
         LIBNGHTTP2_SYS_USE_PKG_CONFIG to build the bundled copy."
      );
    }
    Err(err) => {
      println!(
        "cargo:warning=libnghttp2: system library requested but not found \
         ({err}); falling back to the bundled build"
      );
      false
    }
  }
}

// Whether the LIBNGHTTP2_SYS_USE_PKG_CONFIG environment variable asks for the
// system library (which is then required) with a truthy value.
fn system_linking_required() -> bool {
  match env::var("LIBNGHTTP2_SYS_USE_PKG_CONFIG") {
    Ok(value) => {
      let value = value.trim();
      !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
    }
    Err(_) => false,
  }
}

// Whether the `system` Cargo feature (which tries the system library, falling
// back to the bundled copy) is enabled. Features reach build scripts as
// CARGO_FEATURE_* variables.
fn system_linking_requested_by_feature() -> bool {
  env::var_os("CARGO_FEATURE_SYSTEM").is_some()
}

fn emit_rerun_if_changed() {
  // Build script itself
  println!("cargo:rerun-if-changed=build.rs");

  // Template files
  println!(
    "cargo:rerun-if-changed=nghttp2/lib/includes/nghttp2/nghttp2ver.h.in"
  );
  println!("cargo:rerun-if-changed=nghttp2/lib/libnghttp2.pc.in");

  // Header files (public)
  println!("cargo:rerun-if-changed=nghttp2/lib/includes/nghttp2/nghttp2.h");

  // Source files
  for source in SOURCES {
    println!("cargo:rerun-if-changed={}", source);
  }

  // Header dependencies (internal)
  for header in HEADERS {
    println!("cargo:rerun-if-changed={}", header);
  }
}

struct NgHttp2Version {
  string: String,
  major: u32,
  minor: u32,
  patch: u32,
}

fn parse_nghttp2_version() -> NgHttp2Version {
  let version_str =
    env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION not set");

  let parts: Vec<u32> = version_str
    .split('.')
    .take(3)
    .map(|s| {
      // Strip any pre-release suffixes like "-rc.0"
      let numeric_part = s.split('-').next().unwrap_or(s);
      numeric_part.parse().expect("Invalid version number")
    })
    .collect();

  assert_eq!(parts.len(), 3, "Version must have 3 components");

  NgHttp2Version {
    string: version_str.to_string(),
    major: parts[0],
    minor: parts[1],
    patch: parts[2],
  }
}

fn setup_directories(include_dir: &Path, lib_dir: &Path) {
  fs::create_dir_all(include_dir.join("nghttp2"))
    .expect("Failed to create include directory");
  fs::create_dir_all(lib_dir.join("pkgconfig"))
    .expect("Failed to create lib/pkgconfig directory");
}

// Generate nghttp2ver.h from template
fn generate_version_header(include_dir: &Path, version: &NgHttp2Version) {
  let template =
    fs::read_to_string("nghttp2/lib/includes/nghttp2/nghttp2ver.h.in")
      .expect("Failed to read nghttp2ver.h.in template");

  let version_num = format!(
    "0x{:02x}{:02x}{:02x}",
    version.major, version.minor, version.patch
  );

  let header = template
    .replace("@PACKAGE_VERSION@", &version.string)
    .replace("@PACKAGE_VERSION_NUM@", &version_num);

  fs::write(include_dir.join("nghttp2/nghttp2ver.h"), header)
    .expect("Failed to write nghttp2ver.h");
}

fn copy_main_header(include_dir: &Path) {
  fs::copy(
    "nghttp2/lib/includes/nghttp2/nghttp2.h",
    include_dir.join("nghttp2/nghttp2.h"),
  )
  .expect("Failed to copy nghttp2.h");
}

fn build_nghttp2(target: &str, include_dir: &Path, lib_dir: &Path) {
  let mut build = cc::Build::new();

  build.include("nghttp2/lib/includes").include(include_dir);
  add_source_files(&mut build);
  build
    .warnings(false)
    .define("NGHTTP2_STATICLIB", None)
    .define("HAVE_NETINET_IN", None)
    .define("HAVE_TIME_H", None)
    .out_dir(lib_dir);
  configure_platform(&mut build, target);

  build.compile("nghttp2");
}

fn add_source_files(build: &mut cc::Build) {
  for source in SOURCES {
    build.file(source);
  }
}

fn configure_platform(build: &mut cc::Build, target: &str) {
  if target.contains("windows") {
    // MSVC doesn't have ssize_t, define it based on pointer width
    if target.contains("msvc") {
      let pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
        .expect("CARGO_CFG_TARGET_POINTER_WIDTH not set");

      match pointer_width.as_str() {
        "64" => build.define("ssize_t", "int64_t"),
        "32" => build.define("ssize_t", "int32_t"),
        width => panic!("Unsupported pointer width: {}", width),
      };
    }
  } else {
    build.define("HAVE_ARPA_INET_H", None);
  }
}

fn generate_pkgconfig(
  install_dir: &Path,
  include_dir: &Path,
  lib_dir: &Path,
  version: &NgHttp2Version,
) {
  let template = fs::read_to_string("nghttp2/lib/libnghttp2.pc.in")
    .expect("Failed to read libnghttp2.pc.in template");

  let pkgconfig = template
    .replace("@prefix@", install_dir.to_str().unwrap())
    .replace("@exec_prefix@", "")
    .replace("@libdir@", lib_dir.to_str().unwrap())
    .replace("@includedir@", include_dir.to_str().unwrap())
    .replace("@VERSION@", &version.string);

  fs::write(lib_dir.join("pkgconfig/libnghttp2.pc"), pkgconfig)
    .expect("Failed to write libnghttp2.pc");
}

fn generate_bindings(out_dir: &Path, include_dir: &Path) {
  let header_path = include_dir.join("nghttp2/nghttp2.h");

  let mut clang_args = vec![
    format!("-I{}", include_dir.display()),
    "-Inghttp2/lib/includes".to_string(),
  ];
  clang_args.extend(msvc_ssize_t_clang_arg());

  write_bindings(out_dir, header_path.to_str().unwrap(), &clang_args);
}

// Generate bindings against a system-provided libnghttp2 header. A small
// wrapper header is used so the system include directories resolve
// <nghttp2/nghttp2.h>.
fn generate_bindings_system(out_dir: &Path, include_paths: &[PathBuf]) {
  let wrapper = out_dir.join("nghttp2_wrapper.h");
  fs::write(&wrapper, "#include <nghttp2/nghttp2.h>\n")
    .expect("Failed to write bindgen wrapper header");

  let mut clang_args: Vec<String> = include_paths
    .iter()
    .map(|path| format!("-I{}", path.display()))
    .collect();
  clang_args.extend(msvc_ssize_t_clang_arg());

  write_bindings(out_dir, wrapper.to_str().unwrap(), &clang_args);
}

// On Windows MSVC, clang/bindgen needs ssize_t defined.
fn msvc_ssize_t_clang_arg() -> Option<String> {
  let target = env::var("TARGET").expect("TARGET not set");
  if !(target.contains("windows") && target.contains("msvc")) {
    return None;
  }

  let pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
    .expect("CARGO_CFG_TARGET_POINTER_WIDTH not set");

  let ssize_t_def = match pointer_width.as_str() {
    "64" => "ssize_t=long long",
    "32" => "ssize_t=long",
    width => panic!("Unsupported pointer width: {}", width),
  };

  Some(format!("-D{}", ssize_t_def))
}

fn write_bindings(out_dir: &Path, header: &str, clang_args: &[String]) {
  let mut builder = bindgen::Builder::default().header(header);
  for arg in clang_args {
    builder = builder.clang_arg(arg);
  }

  // Note: We don't use CargoCallbacks here because it would emit
  // rerun-if-changed for generated headers in OUT_DIR (whose path changes
  // between builds) and system headers. We manually emit rerun-if-changed
  // for the source files in emit_rerun_if_changed() instead.
  let bindings = builder
    // Only include nghttp2 symbols
    .allowlist_function("nghttp2_.*")
    .allowlist_type("nghttp2_.*")
    .allowlist_var("NGHTTP2_.*")
    // Generate enum constants with consistent types across platforms.
    // Without this, C enums become i32 on MSVC (C standard: enum is int)
    // but u32 on GCC/Clang (where the underlying type matches the values).
    .default_enum_style(bindgen::EnumVariation::Consts)
    .translate_enum_integer_types(true)
    // Opaque types that should not derive Copy
    .opaque_type("nghttp2_session")
    .opaque_type("nghttp2_rcbuf")
    .opaque_type("nghttp2_session_callbacks")
    .opaque_type("nghttp2_option")
    .opaque_type("nghttp2_hd_deflater")
    .opaque_type("nghttp2_hd_inflater")
    .opaque_type("nghttp2_stream")
    .layout_tests(false)
    //.generate_comments(false)
    .prepend_enum_name(false)
    .blocklist_function(".*vprintf.*")
    .blocklist_type(".*va_list.*")
    .blocklist_type("nghttp2_debug_vprintf_callback")
    .generate()
    .expect("Failed to generate bindings");

  bindings
    .write_to_file(out_dir.join("bindings.rs"))
    .expect("Failed to write bindings");
}
