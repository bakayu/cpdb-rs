# cpdb-sys

[![Crates.io](https://img.shields.io/crates/v/cpdb-sys.svg)](https://crates.io/crates/cpdb-sys)
[![Documentation](https://docs.rs/cpdb-sys/badge.svg)](https://docs.rs/cpdb-sys)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Raw FFI bindings for the Common Print Dialog Backends C library ([cpdb-libs](https://github.com/OpenPrinting/cpdb-libs)).

This crate is automatically generated using `bindgen` and contains raw, unsafe declarations for the complete CPDB C API (`libcpdb` and `libcpdb-frontend`).

> [!NOTE]
> **Looking for a safe Rust API?** Most users should depend on the high-level safe async crate [cpdb-rs](https://crates.io/crates/cpdb-rs) instead, which provides pure-Rust D-Bus capabilities without C library dependencies.

---

## Supported Platforms

| Target | Status | Notes |
|---|---|---|
| Linux (any glibc distro) | ✅ Fully supported | The intended target. CI runs on Ubuntu. |
| macOS | ⚠️ Headers-only | Bindgen can parse the headers and the crate compiles with `CPDB_NO_LINK=1`, but linking requires Linux D-Bus. Useful only for compile-checking. |
| Windows | ❌ Not supported | `cpdb-libs` has no Windows port (D-Bus / GLib stack). Compilation will hard-fail with a `compile_error!`. Develop inside [WSL Ubuntu](https://learn.microsoft.com/windows/wsl/install). |

---

## Prerequisites

### cpdb-libs (≥ 3.0)

`cpdb-sys` targets the cpdb-libs **3.x ABI** and links against `libcpdb` and `libcpdb-frontend`.

> [!IMPORTANT]
> **Distro packages may be too old.** As of mid-2026, Debian / Ubuntu ship `cpdb-libs` **2.0~b5** in `libcpdb-dev`. That is incompatible with this crate — installing it leaves you with both `libcpdb.so.2` (from apt) and `libcpdb.so.3` (from source) and the linker picks the wrong one. Either:
>
> 1. **Build from source** (recommended until distros catch up), or
> 2. Verify your package gives `libcpdb.so.3.*` with `ls /usr/lib*/libcpdb.so.*` before relying on it.

**Build cpdb-libs 3.x from source:**

```bash
sudo apt-get install -y \
    build-essential pkg-config autoconf automake libtool libtool-bin \
    gettext autopoint libglib2.0-dev libdbus-1-dev libclang-dev \
    libcups2-dev cups libavahi-common-dev libavahi-client-dev

# If you previously installed apt's older libcpdb*, remove it first:
sudo apt-get remove --purge libcpdb-dev libcpdb2t64 2>/dev/null

git clone --depth=1 https://github.com/OpenPrinting/cpdb-libs.git
cd cpdb-libs
./autogen.sh || autoreconf -fi
./configure --prefix=/usr
make -j"$(nproc)"
sudo make install
sudo ldconfig
```

Fedora / RHEL: install `cpdb-libs-devel` from a 3.x-shipping repository, or build from source the same way.

### libclang (for bindgen)

`bindgen` needs `libclang` at build time to parse C headers. On Debian/Ubuntu:

```bash
sudo apt-get install -y libclang-dev clang
```

---

## Build Configuration

If the library is installed to a non-standard location where `pkg-config` cannot locate it, you can specify the prefix path by setting the `CPDB_LIBS_PATH` environment variable:

```bash
export CPDB_LIBS_PATH=/path/to/custom/prefix
```
