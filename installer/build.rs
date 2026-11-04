//! 把 NiuMa.exe 压缩后嵌进安装程序，读出主程序版本号，并链接清单资源。
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let here = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 要打包的程序，默认取 dist\NiuMa.exe（scripts\build.ps1 会先生成它）。
    println!("cargo:rerun-if-env-changed=NIUMA_PAYLOAD");
    let payload = env::var_os("NIUMA_PAYLOAD")
        .map(PathBuf::from)
        .unwrap_or_else(|| here.join("../dist/NiuMa.exe"));
    println!("cargo:rerun-if-changed={}", payload.display());
    let bytes = fs::read(&payload).unwrap_or_else(|e| {
        panic!("读不到要打包的程序 {}：{e}。请先运行 scripts\\build.ps1", payload.display())
    });
    let packed = miniz_oxide::deflate::compress_to_vec(&bytes, 10);
    fs::write(out.join("payload.bin"), packed).unwrap();

    // 版本号跟随主程序的 Cargo.toml，第一条 `version = "..."` 就是包版本。
    let root_toml = here.join("../Cargo.toml");
    println!("cargo:rerun-if-changed={}", root_toml.display());
    let text = fs::read_to_string(&root_toml).unwrap();
    let version = text
        .lines()
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
        .expect("主程序 Cargo.toml 里没有 version");
    println!("cargo:rustc-env=NIUMA_VERSION={version}");

    // 清单：不提权运行、启用新版通用控件（TaskDialog 需要）、高 DPI 清晰显示。
    let rc = here.join("res/setup.rc");
    println!("cargo:rerun-if-changed={}", rc.display());
    println!("cargo:rerun-if-changed={}", here.join("res/setup.manifest").display());
    let obj = out.join("setup-res.o");
    let status = Command::new("llvm-windres")
        .args(["--no-preprocess", "-O", "coff"])
        .arg(&rc)
        .arg(&obj)
        .status()
        .expect("找不到 llvm-windres，请先运行 scripts\\env.ps1");
    assert!(status.success(), "llvm-windres 编译清单失败");
    println!("cargo:rustc-link-arg-bins={}", obj.display());
}
