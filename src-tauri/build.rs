fn main() {
    println!("cargo:rerun-if-changed=network-backend/");
    tauri_build::build();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    let mut c_build = cc::Build::new();
    c_build
        .files(&[
            "network-backend/nanopb/pb_common.c",
            "network-backend/nanopb/pb_decode.c",
            "network-backend/nanopb/pb_encode.c",
            "network-backend/proto/networking.pb.c",
        ])
        .include("network-backend/nanopb/")
        .include("network-backend/proto/");

    if target_os == "windows" {
        c_build.define("_WIN32_WINNT", "0x0600");
    }

    c_build.compile("network_backend_c");

    let mut cpp_build = cc::Build::new();
    cpp_build
        .cpp(true)
        .std("c++20")
        .files(&[
            "network-backend/networkBackend.cpp",
            "network-backend/NetworkSession.cpp",
            "network-backend/codes.cpp",
            "network-backend/sockopt.cpp",
        ])
        .include("network-backend/")
        .include("network-backend/nanopb/")
        .include("network-backend/proto/");

    if target_os == "windows" {
        cpp_build.define("_WIN32_WINNT", "0x0600");
    }

    cpp_build.compile("network_backend_cpp");

    if target_os == "macos" {
        println!("cargo:rustc-link-lib=resolv");
    } else if target_os == "windows" {
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=iphlpapi");
    }
}
