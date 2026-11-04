# Third-party notices / 第三方声明

The original code of NiuMa Clock is MIT licensed (see [LICENSE](LICENSE)). Third-party material keeps its own license. Original license files are kept, unmodified, in [`third-party-licenses/`](third-party-licenses/).

本项目自有代码采用 MIT 许可证。第三方组件保留各自的许可证，原文未经修改保存在 `third-party-licenses/` 下。

## Rust crates

The release binaries (`NiuMa.exe`, and `NiuMa-Setup.exe` which embeds it) statically link the crates below. The list comes from `cargo metadata` for the `x86_64-pc-windows-gnullvm` target, normal and build dependencies of both the app and the installer; dev-only dependencies are excluded.

Where a crate offers a choice, this project elects the MIT license if it is one of the options. Otherwise it elects BSD-3-Clause for `moxcms` and `pxfm`, and Apache-2.0 for `self_cell`. Crates whose license is an `AND` expression are used under all of the listed terms. `epaint_default_fonts` embeds the Ubuntu-Light (Ubuntu Font Licence 1.0), Hack (MIT, with the Bitstream Vera and DejaVu notices), Noto Emoji (OFL-1.1) and emoji-icon-font (MIT, plus the `egui-icons` subset of it) fonts; their notices are in that crate's directory.

† The published crate archive contains no license file. The license files were taken from the upstream repository at the commit recorded in the crate's `.cargo_vcs_info.json` (for the egui crates, the `0.37.0` release tag).

| Crate | Version | License expression | Notices |
| --- | --- | --- | --- |
| accesskit | 0.25.2 | MIT OR Apache-2.0 | `rust-crates/accesskit-0.25.2` † |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | `rust-crates/adler2-2.0.1` |
| ahash | 0.8.12 | MIT OR Apache-2.0 | `rust-crates/ahash-0.8.12` |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | `rust-crates/arrayvec-0.7.8` |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | `rust-crates/autocfg-1.5.1` |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | `rust-crates/bitflags-2.13.2` |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | `rust-crates/bytemuck-1.25.2` |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT | `rust-crates/bytemuck_derive-1.12.1` |
| byteorder-lite | 0.1.0 | Unlicense OR MIT | `rust-crates/byteorder-lite-0.1.0` |
| cfg_aliases | 0.2.2 | MIT | `rust-crates/cfg_aliases-0.2.2` |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | `rust-crates/cfg-if-1.0.5` |
| chrono | 0.4.45 | MIT OR Apache-2.0 | `rust-crates/chrono-0.4.45` |
| color | 0.3.3 | Apache-2.0 OR MIT | `rust-crates/color-0.3.3` |
| crc32fast | 1.5.3 | MIT OR Apache-2.0 | `rust-crates/crc32fast-1.5.3` |
| crossbeam-channel | 0.5.17 | MIT OR Apache-2.0 | `rust-crates/crossbeam-channel-0.5.17` |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | `rust-crates/crossbeam-utils-0.8.23` |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib | `rust-crates/cursor-icon-1.2.0` |
| document-features | 0.2.12 | MIT OR Apache-2.0 | `rust-crates/document-features-0.2.12` |
| dpi | 0.1.2 | Apache-2.0 AND MIT | `rust-crates/dpi-0.1.2` |
| ecolor | 0.37.0 | MIT OR Apache-2.0 | `rust-crates/ecolor-0.37.0` † |
| eframe | 0.37.0 | MIT OR Apache-2.0 | `rust-crates/eframe-0.37.0` † |
| egui | 0.37.0 | MIT OR Apache-2.0 | `rust-crates/egui-0.37.0` † |
| egui_glow | 0.37.0 | MIT OR Apache-2.0 | `rust-crates/egui_glow-0.37.0` † |
| egui-winit | 0.37.0 | MIT OR Apache-2.0 | `rust-crates/egui-winit-0.37.0` † |
| either | 1.19.0 | MIT OR Apache-2.0 | `rust-crates/either-1.19.0` |
| emath | 0.37.0 | MIT OR Apache-2.0 | `rust-crates/emath-0.37.0` † |
| epaint | 0.37.0 | MIT OR Apache-2.0 | `rust-crates/epaint-0.37.0` † |
| epaint_default_fonts | 0.37.0 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | `rust-crates/epaint_default_fonts-0.37.0` † |
| euclid | 0.22.14 | MIT OR Apache-2.0 | `rust-crates/euclid-0.22.14` |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | `rust-crates/fdeflate-0.3.7` |
| fearless_simd | 0.4.1 | Apache-2.0 OR MIT | `rust-crates/fearless_simd-0.4.1` |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | `rust-crates/flate2-1.1.10` |
| foldhash | 0.2.0 | Zlib | `rust-crates/foldhash-0.2.0` |
| font-types | 0.12.7 | MIT OR Apache-2.0 | `rust-crates/font-types-0.12.7` |
| gl_generator | 0.14.0 | Apache-2.0 | `rust-crates/gl_generator-0.14.0` † |
| glifo | 0.3.0 | Apache-2.0 OR MIT | `rust-crates/glifo-0.3.0` |
| global-hotkey | 0.9.0 | Apache-2.0 OR MIT | `rust-crates/global-hotkey-0.9.0` |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib | `rust-crates/glow-0.17.0` |
| glutin | 0.32.3 | Apache-2.0 | `rust-crates/glutin-0.32.3` |
| glutin_egl_sys | 0.7.1 | Apache-2.0 | `rust-crates/glutin_egl_sys-0.7.1` |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 | `rust-crates/glutin_wgl_sys-0.6.1` |
| glutin-winit | 0.5.0 | MIT | `rust-crates/glutin-winit-0.5.0` |
| guillotiere | 0.7.0 | MIT/Apache-2.0 | `rust-crates/guillotiere-0.7.0` |
| harfrust | 0.12.0 | MIT | `rust-crates/harfrust-0.12.0` |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | `rust-crates/hashbrown-0.17.1` |
| image | 0.25.10 | MIT OR Apache-2.0 | `rust-crates/image-0.25.10` |
| itertools | 0.15.0 | MIT OR Apache-2.0 | `rust-crates/itertools-0.15.0` |
| itoa | 1.0.18 | MIT OR Apache-2.0 | `rust-crates/itoa-1.0.18` |
| keyboard-types | 0.8.3 | MIT OR Apache-2.0 | `rust-crates/keyboard-types-0.8.3` |
| khronos_api | 3.1.0 | Apache-2.0 | `rust-crates/khronos_api-3.1.0` † |
| kurbo | 0.13.1 | Apache-2.0 OR MIT | `rust-crates/kurbo-0.13.1` |
| libloading | 0.8.9 | ISC | `rust-crates/libloading-0.8.9` |
| linebender_resource_handle | 0.1.1 | Apache-2.0 OR MIT | `rust-crates/linebender_resource_handle-0.1.1` |
| litrs | 1.0.1 | MIT OR Apache-2.0 | `rust-crates/litrs-1.0.1` |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | `rust-crates/lock_api-0.4.14` |
| log | 0.4.35 | MIT OR Apache-2.0 | `rust-crates/log-0.4.35` |
| memchr | 2.8.3 | Unlicense OR MIT | `rust-crates/memchr-2.8.3` |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | `rust-crates/miniz_oxide-0.9.1` |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | `rust-crates/miniz_oxide-0.8.9` |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 | `rust-crates/moxcms-0.8.1` |
| muda | 0.21.2 | Apache-2.0 OR MIT | `rust-crates/muda-0.21.2` |
| nohash-hasher | 0.2.0 | Apache-2.0 OR MIT | `rust-crates/nohash-hasher-0.2.0` |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | `rust-crates/num-traits-0.2.19` |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | `rust-crates/once_cell-1.21.4` |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | `rust-crates/parking_lot-0.12.5` |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | `rust-crates/parking_lot_core-0.9.12` |
| peniko | 0.6.1 | Apache-2.0 OR MIT | `rust-crates/peniko-0.6.1` |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | `rust-crates/pin-project-lite-0.2.17` |
| png | 0.18.1 | MIT OR Apache-2.0 | `rust-crates/png-0.18.1` |
| polycool | 0.4.0 | MIT OR Apache-2.0 | `rust-crates/polycool-0.4.0` |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | `rust-crates/proc-macro2-1.0.107` |
| profiling | 1.0.18 | MIT OR Apache-2.0 | `rust-crates/profiling-1.0.18` † |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 | `rust-crates/pxfm-0.1.30` |
| quote | 1.0.47 | MIT OR Apache-2.0 | `rust-crates/quote-1.0.47` |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | `rust-crates/raw-window-handle-0.6.2` |
| read-fonts | 0.41.0 | MIT OR Apache-2.0 | `rust-crates/read-fonts-0.41.0` |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | `rust-crates/scopeguard-1.2.0` |
| self_cell | 1.3.0 | Apache-2.0 OR GPL-2.0-only | `rust-crates/self_cell-1.3.0` |
| serde | 1.0.229 | MIT OR Apache-2.0 | `rust-crates/serde-1.0.229` |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | `rust-crates/serde_core-1.0.229` |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | `rust-crates/serde_derive-1.0.229` |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | `rust-crates/serde_json-1.0.154` |
| simd-adler32 | 0.3.10 | MIT | `rust-crates/simd-adler32-0.3.10` |
| skrifa | 0.44.0 | MIT OR Apache-2.0 | `rust-crates/skrifa-0.44.0` |
| smallvec | 1.16.4 | MIT OR Apache-2.0 | `rust-crates/smallvec-1.16.4` |
| smol_str | 0.2.2 | MIT OR Apache-2.0 | `rust-crates/smol_str-0.2.2` |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | `rust-crates/static_assertions-1.1.0` |
| syn | 3.0.9 | MIT OR Apache-2.0 | `rust-crates/syn-3.0.9` |
| syn | 2.0.119 | MIT OR Apache-2.0 | `rust-crates/syn-2.0.119` |
| tauri-winrt-notification | 0.8.1 | MIT OR Apache-2.0 | `rust-crates/tauri-winrt-notification-0.8.1` |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | `rust-crates/thiserror-2.0.21` |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | `rust-crates/thiserror-impl-2.0.21` |
| tracing | 0.1.44 | MIT | `rust-crates/tracing-0.1.44` |
| tracing-core | 0.1.36 | MIT | `rust-crates/tracing-core-0.1.36` |
| tray-icon | 0.26.1 | MIT OR Apache-2.0 | `rust-crates/tray-icon-0.26.1` |
| unicode-general-category | 1.1.0 | Apache-2.0 | `rust-crates/unicode-general-category-1.1.0` |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | `rust-crates/unicode-ident-1.0.27` |
| unicode-properties | 0.1.5 | (MIT OR Apache-2.0) AND Unicode-3.0 | `rust-crates/unicode-properties-0.1.5` |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | `rust-crates/unicode-segmentation-1.13.3` |
| uuid | 1.29.0 | Apache-2.0 OR MIT | `rust-crates/uuid-1.29.0` |
| vello_common | 0.2.0 | Apache-2.0 OR MIT | `rust-crates/vello_common-0.2.0` |
| vello_cpu | 0.2.0 | Apache-2.0 OR MIT | `rust-crates/vello_cpu-0.2.0` |
| version_check | 0.9.5 | MIT/Apache-2.0 | `rust-crates/version_check-0.9.5` |
| web-time | 1.1.0 | MIT OR Apache-2.0 | `rust-crates/web-time-1.1.0` |
| windows | 0.62.2 | MIT OR Apache-2.0 | `rust-crates/windows-0.62.2` |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | `rust-crates/windows_x86_64_gnullvm-0.52.6` |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | `rust-crates/windows-collections-0.3.2` |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | `rust-crates/windows-core-0.62.2` |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | `rust-crates/windows-future-0.3.2` |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | `rust-crates/windows-implement-0.60.2` |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | `rust-crates/windows-interface-0.59.3` |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | `rust-crates/windows-link-0.2.1` |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | `rust-crates/windows-numerics-0.3.1` |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | `rust-crates/windows-result-0.4.1` |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | `rust-crates/windows-strings-0.5.1` |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | `rust-crates/windows-sys-0.61.2` |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 | `rust-crates/windows-sys-0.59.0` |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | `rust-crates/windows-sys-0.52.0` |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | `rust-crates/windows-targets-0.52.6` |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | `rust-crates/windows-threading-0.2.1` |
| windows-version | 0.1.7 | MIT OR Apache-2.0 | `rust-crates/windows-version-0.1.7` |
| winit | 0.30.13 | Apache-2.0 | `rust-crates/winit-0.30.13` |
| xml-rs | 0.8.29 | MIT | `rust-crates/xml-rs-0.8.29` |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | `rust-crates/zerocopy-0.8.63` |
| zlib-rs | 0.6.8 | Zlib | `rust-crates/zlib-rs-0.6.8` |
| zmij | 1.0.23 | MIT | `rust-crates/zmij-1.0.23` |

## Toolchain runtime

- **Rust standard library** (statically linked): [`third-party-licenses/rust-toolchain/`](third-party-licenses/rust-toolchain/) contains `COPYRIGHT-library.html` from rustc 1.99.1 and the license texts it references.
- **mingw-w64 runtime** (CRT startup code and import stubs, statically linked via `+crt-static`): [`third-party-licenses/mingw-w64-runtime/`](third-party-licenses/mingw-w64-runtime/), taken from the llvm-mingw toolchain used for linking. The winpthreads notice is included in case it is linked.
- **LLVM libunwind and compiler-rt builtins** (statically linked by the gnullvm target): Apache-2.0 with LLVM Exceptions, [`third-party-licenses/llvm/LICENSE.TXT`](third-party-licenses/llvm/LICENSE.TXT).

## Not bundled

- Chinese text is drawn with fonts already installed in Windows (`C:\Windows\Fonts`), loaded at run time. No Windows fonts are copied into the program.
- The installer uses Windows' own task dialog, shell link and registry APIs. No Inno Setup, NSIS or WiX code is included.
- The tray and window icon (a cartoon ox head) is drawn by this project's own code (`src/icon.rs`) and is covered by the MIT license.