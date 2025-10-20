use std::env;
use std::path::PathBuf;

const CPP_REPO_URL: &str = "https://github.com/jsulmont/spark-plug_cpp.git";
const CPP_REPO_BRANCH: &str = "main"; // Use main branch (or pin to a tag like "v0.1.0")

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let cpp_repo_dir = out_dir.join("spark-plug_cpp");
    if !cpp_repo_dir.exists() {
        println!("Cloning sparkplug_cpp from GitHub...");
        let mut builder = git2::build::RepoBuilder::new();
        builder.branch(CPP_REPO_BRANCH);
        builder
            .clone(CPP_REPO_URL, &cpp_repo_dir)
            .expect("Failed to clone sparkplug_cpp repository");
    }

    println!("Building sparkplug_cpp C library...");
    let cpp_build_dir = out_dir.join("cpp_build");

    // Detect system C/C++ compiler matching the C++ project's CMakeLists.txt expectations
    // macOS: Use Homebrew LLVM (C++23 support with libc++)
    // Linux: Use system clang (preferably clang-18)
    let c_compiler = env::var("CMAKE_C_COMPILER").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "/opt/homebrew/opt/llvm/bin/clang".to_string()
        } else {
            "clang".to_string()
        }
    });

    let cxx_compiler = env::var("CMAKE_CXX_COMPILER").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "/opt/homebrew/opt/llvm/bin/clang++".to_string()
        } else {
            "clang++".to_string()
        }
    });

    let dst = cmake::Config::new(&cpp_repo_dir)
        .define("BUILD_SHARED_LIBS", "ON")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("CMAKE_EXPORT_COMPILE_COMMANDS", "ON")
        .define("CMAKE_C_COMPILER", &c_compiler)
        .define("CMAKE_CXX_COMPILER", &cxx_compiler)
        .out_dir(&cpp_build_dir)
        .build_target("sparkplug_c")
        .build();

    let lib_dir = dst.join("lib");
    let lib64_dir = dst.join("lib64");
    let build_lib_dir = cpp_build_dir.join("build").join("src");

    let link_search_path = if lib_dir.exists()
        && (lib_dir.join("libsparkplug_c.dylib").exists()
            || lib_dir.join("libsparkplug_c.so").exists())
    {
        lib_dir
    } else if lib64_dir.exists() {
        lib64_dir
    } else if build_lib_dir.exists() {
        build_lib_dir
    } else {
        // Fallback to build/src which is the typical location
        cpp_build_dir.join("build").join("src")
    };

    println!(
        "cargo:rustc-link-search=native={}",
        link_search_path.display()
    );
    println!("cargo:rustc-link-lib=dylib=sparkplug_c");

    let header_path = cpp_repo_dir.join("include/sparkplug/sparkplug_c.h");

    println!("cargo:rerun-if-changed=build.rs");

    let bindings = bindgen::Builder::default()
        .header(header_path.to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .blocklist_type("std::.*")
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(false)
        .use_core()
        .clang_arg("-xc")
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("Sparkplug C++ library built successfully!");
}
