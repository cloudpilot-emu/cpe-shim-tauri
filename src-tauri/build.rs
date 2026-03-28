fn main() {
    tauri_build::build();

    cc::Build::new()
        .files(&[
            "network-backend/nanopb/pb_common.c",
            "network-backend/nanopb/pb_decode.c",
            "network-backend/nanopb/pb_encode.c",
            "network-backend/proto/networking.pb.c",
        ])
        .include("network-backend/nanopb/")
        .include("network-backend/proto/")
        .compile("network_backend_c");

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .files(&[
            "network-backend/networkBackend.cpp",
            "network-backend/NetworkSession.cpp",
            "network-backend/codes.cpp",
            "network-backend/sockopt.cpp",
        ])
        .include("network-backend/")
        .include("network-backend/nanopb/")
        .include("network-backend/proto/")
        .compile("network_backend_cpp");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=resolv");
    }
}
