# cpdb-rs

[![Crates.io](https://img.shields.io/crates/v/cpdb-rs.svg)](https://crates.io/crates/cpdb-rs)
[![Documentation](https://docs.rs/cpdb-rs/badge.svg)](https://docs.rs/cpdb-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Safe, native Rust async bindings for the Common Print Dialog Backends ([CPDB](https://github.com/OpenPrinting/cpdb-libs)) D-Bus interface.

## Overview

cpdb-rs lets Rust applications communicate with CPDB print backends (like `cpdb-backend-cups`) directly over D-Bus without requiring any C dependencies.

The library uses [`zbus`](https://crates.io/crates/zbus) and [`tokio`](https://crates.io/crates/tokio) to provide a fully asynchronous, memory-safe, and pure-Rust implementation of the CPDB client protocol.

## Features

- **Pure Rust D-Bus Client:** No `libcpdb-dev` or C compiler needed! Everything runs natively over D-Bus using `zbus`.
- **Async First:** All methods are `async` and powered by Tokio.
- **Live Discovery:** Subscribe to a native Rust `Stream` for real-time printer additions, removals, and state changes.
- **Activation Retry:** Gracefully retries printer discovery to handle D-Bus activation race conditions.
- **Keep-Alive Management:** Automatically pings backends to keep them active in the background.
- *(Optional)* **Legacy C-FFI bindings** available via the `ffi` feature flag.

## Supported platforms

The default (`zbus-backend`) install is pure Rust and does not link
`cpdb-libs`. Requirements are minimal and Unix-only.

| Target | `zbus-backend` (default) | `ffi` (opt-in) |
|---|---|---|
| Linux (glibc) | ✅ Fully supported. CI runs on Ubuntu. | ✅ Fully supported. |
| macOS | ✅ Compiles. Runtime requires a session bus and a CPDB backend. | ⚠️ Headers-only. Bindgen parses the headers and the crate compiles with `CPDB_NO_LINK=1`, but linking requires Linux D-Bus — useful only for compile-checks. |
| Windows | ❌ Not supported. Develop inside [WSL Ubuntu](https://learn.microsoft.com/windows/wsl/install) — the repository on `/mnt/c/…` is reachable from WSL without copying. | ❌ Not supported. Use WSL. |

At runtime, whichever backend you pick, your system must have at least
one CPDB backend service installed (e.g. `cpdb-backend-cups`) for
printer discovery to return anything.

### Prerequisites (default `zbus-backend`)

- Rust **1.85+** (2024 edition).
- A running D-Bus session bus.
- At least one CPDB backend service reachable on that bus for anything
  useful to happen at runtime.

That's it — no C compiler, no `libcpdb-dev`, no `bindgen`. Skip straight
to [Installation](#installation).

### Prerequisites (for the `ffi` feature)

Everything below is only needed if you opt in with
`--features ffi` (or `default-features = false, features = ["ffi"]`).

#### cpdb-libs (≥ 3.0)

The `ffi` feature targets the cpdb-libs **3.x ABI**.

> ⚠️ **Distro packages may be too old.** As of mid-2026, Debian / Ubuntu
> ship cpdb-libs **2.0~b5** in `libcpdb-dev`. That is incompatible with
> this crate — installing it leaves you with both `libcpdb.so.2`
> (from apt) and `libcpdb.so.3` (from source) and the linker picks the
> wrong one. Either:
>
> 1. **Build from source** (recommended until distros catch up), or
> 2. Verify your package gives `libcpdb.so.3.*` with
>    `ls /usr/lib*/libcpdb.so.*` before relying on it.

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

Fedora / RHEL: install `cpdb-libs-devel` from a 3.x-shipping repository,
or build from source the same way.

#### libclang

bindgen needs libclang at build time. On Debian/Ubuntu:

```bash
sudo apt-get install -y libclang-dev clang
```

## Installation

Default install — pure Rust, no C dependencies:

```toml
[dependencies]
cpdb-rs = "0.1.0"
tokio  = { version = "1.0", features = ["full"] }
```

Legacy synchronous C-FFI wrappers (requires `cpdb-libs` 3.x and
`libclang` at build time — see [Prerequisites (for the `ffi`
feature)](#prerequisites-for-the-ffi-feature)):

```toml
[dependencies]
cpdb-rs = { version = "0.1.0", default-features = false, features = ["ffi"] }
```

Enabling both features at once is supported: the default `zbus-backend`
API and the FFI wrappers coexist.

## Quick start

```rust
use cpdb_rs::CpdbClient;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    // Connect to D-Bus and auto-activate available CPDB backends
    let client = CpdbClient::new().await?;
    println!("Connected to {} backend(s).\n", client.backend_count());

    // Retrieve all active printers
    let printers = client.get_all_printers().await?;
    for p in &printers {
        println!("Printer: {} (ID: {})", p.name, p.id);
        println!("  Make & Model: {}", p.make_model);
        println!("  State: {}", p.state);
        println!("  Accepts Jobs: {}", p.accepting_jobs);
    }

    Ok(())
}
```

## Examples

### Fetching Printer Options and Media Sizes

```rust
use cpdb_rs::CpdbClient;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;
    let printers = client.get_all_printers().await?;

    if let Some(p) = printers.first() {
        // Fetch specific details using the printer's ID and backend name
        let (options, media) = client.get_printer_details(&p.id, &p.backend).await?;

        println!("Options for {}:", p.name);
        for opt in options {
            println!("  {}: default='{}', choices=[{}]", 
                opt.name, opt.default_value, opt.supported_values.join(", "));
        }
    }
    Ok(())
}
```

### Live Discovery Event Stream

Watch for new printers appearing and disappearing in real-time.

```rust
use cpdb_rs::{CpdbClient, DiscoveryEvent};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;
    
    // Spawn a background task to keep the backends from automatically
    // timing out and exiting after 30 seconds of inactivity.
    let keep_alive_client = client.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            keep_alive_client.keep_alive_all().await;
        }
    });

    let mut stream = client.discovery_stream().await?;
    println!("Listening for printer changes...");

    while let Some(event) = stream.next().await {
        match event {
            DiscoveryEvent::PrinterAdded(snap) => {
                println!("+ Added: {} ({})", snap.name, snap.backend);
            }
            DiscoveryEvent::PrinterRemoved { id, backend } => {
                println!("- Removed: {} ({})", id, backend);
            }
            DiscoveryEvent::PrinterStateChanged { id, state, accepting_jobs, .. } => {
                println!("~ State Changed: {} is now {} (accepting={})", id, state, accepting_jobs);
            }
        }
    }

    Ok(())
}
```

You can run any of the shipped examples against your live D-Bus session:

```bash
cargo run --example discover_printers    # list every printer every backend reports
cargo run --example filter_printers      # apply a Rust-side filter over the snapshot
cargo run --example get_translations     # localise option / choice labels
cargo run --example print_a_document     # end-to-end job submission
```

FFI-flavoured examples are gated on the `ffi` feature:

```bash
cargo run --example ffi_basic_usage         --no-default-features --features ffi
cargo run --example ffi_cli_printer_manager --no-default-features --features ffi
```

## Architecture (zbus Backend)

Instead of linking against `libcpdb.so` and using `bindgen` (which required unsafe C memory management, callbacks, and manual lifetime tracking), `cpdb-rs` now uses `zbus` to speak the D-Bus protocol directly to the print backends. This provides massive benefits:

- **100% Safe Rust:** No raw pointers, no manual memory management, no undefined behavior.
- **Zero C Dependencies:** You don't need `libcpdb-dev` to compile.
- **Async Tokio Integration:** `zbus` integrates perfectly with Tokio, allowing you to await D-Bus calls and use Rust `Stream`s for live discovery events.
- **Activation Retries:** Automatically retries initial calls to handle `UnknownMethod` race conditions when systemd auto-activates D-Bus backends.

## Legacy Architecture (C-FFI)

> [!WARNING]  
> The C-FFI interface is behind the `ffi` feature flag and has been moved to the underlying `cpdb-sys` crate. The default feature is now `zbus-backend`. To continue using `cpdb_rs::Frontend`, update your `Cargo.toml` to: `cpdb-rs = { default-features = false, features = ["ffi"] }`.

If you have legacy code that still requires the synchronous C-FFI wrappers around `cpdb-libs`, they are still available by enabling the `ffi` feature flag in your `Cargo.toml`. See the `ffi` module documentation for details.

```
            ┌───────────────────────────────────────────┐
            │            cpdb_rs::Frontend              │
            │  (D-Bus connection, backend list, hash    │
            │   table of discovered printers)           │
            └─────┬─────────────────────────────────┬───┘
                  │ borrowed                        │ owned
                  │ (lifetime tied to &Frontend)    │ (Drop frees)
                  ▼                                 ▼
       ┌────────────────────┐            ┌──────────────────────┐
       │ Printer<'frontend> │            │ Printer<'static>     │
       │ from get_printer / │            │ from load_from_file  │
       │ find_printer / ... │            └──────────────────────┘
       └────────┬───────────┘
                │
                ├── per-printer settings  (add_setting / clear_setting)
                ├── option lookups        (get_default / get_current / get_option)
                ├── translations         ─ TranslationMap (owned snapshot)
                ├── media                ─ MediaSize, Margins
                └── job submission       (print_file / submit_job / print_fd / print_socket)


                       ┌──────────────────────────┐
                       │     cpdb_rs::Settings     │
                       │  (free-standing serial-   │
                       │   isable settings object) │
                       └──────────────────────────┘
                            ▲
                            │ persisted via save_to_disk / read_from_disk
                            ▼
                   ~/.config/cpdb/ (cpdb-libs-managed location)
```

### Two `add_setting` methods, two scopes (C-FFI)

| Method                    | Scope                                              | Persists across runs?               |
|---------------------------|----------------------------------------------------|-------------------------------------|
| `Printer::add_setting`    | This printer only, in-memory on the printer object | Only if you re-add on each run      |
| `Settings::add_setting`   | Free-standing settings collection                  | Yes, via `Settings::save_to_disk()` |

`Printer::add_setting` is the per-job knob: tweak `copies`, `sides`, etc.
before calling `print_file` / `submit_job`. `Settings` is the global,
serialisable view that cpdb-libs reads back from disk on startup.

### Module map

| Module                | What lives here                                                     |
|-----------------------|----------------------------------------------------------------------|
| `cpdb_rs::client`     | **(zbus)** `CpdbClient` — Main async D-Bus client & discovery logic |
| `cpdb_rs::events`     | **(zbus)** `DiscoveryEvent`, `PrinterSnapshot` for async streams    |
| `cpdb_rs::media`      | **(zbus)** `MediaCollection`, `MediaInfo`, `MarginInfo`             |
| `cpdb_rs::config`     | **(zbus)** `PrinterConfig` for job submission configuration         |
| `cpdb_rs::options`    | `OptionInfo`, `OptionsCollection` (shared across both implementations)|
| `cpdb_rs::error`      | `CpdbError` and the crate-wide `Result` alias                       |
| `cpdb_rs::proxy`      | **(zbus)** Auto-generated zbus proxy trait `PrintBackend`           |
| `cpdb_rs::frontend`   | *(ffi)* `Frontend` — D-Bus lifecycle, discovery, default printer    |
| `cpdb_rs::printer`    | *(ffi)* `Printer`, `Margin/Margins`, `MediaSize`, `TranslationMap`  |
| `cpdb_rs::settings`   | *(ffi)* `Settings`, `Options`, `Media`                              |
| `cpdb_rs::callbacks`  | *(ffi)* Closure trampolines + `PrinterUpdate` enum                  |
| `cpdb_rs::common`     | *(ffi)* `init`, `version`, path/config helpers                      |
| `cpdb_rs::util`       | *(ffi)* Internal `CStr` helpers + the `COptions` C-array builder    |
| `cpdb_rs::ffi`        | *(ffi)* Raw bindgen output; everything `unsafe`                     |

## Ownership model (C-FFI)

`Printer` carries a lifetime tied to the `Frontend` it came from. Borrowed
printers (those returned by `get_printers`, `get_printer`, `find_printer`,
`get_default_printer`, ...) cannot outlive their frontend — the compiler
checks this for you. Owned printers (`Printer::load_from_file`) have a
`'static` lifetime and are freed when dropped.

`Printer` is intentionally **not** `Send` or `Sync`. cpdb-libs does not
lock internally; if you need cross-thread access, wrap the printer in a
`Mutex` (or, more typically, run your printer operations on a single
thread).

`Frontend` is `Send` but **not** `Sync` — for the same reason.

## Error handling

`CpdbError` is `#[non_exhaustive]`, so always include a wildcard arm:

```rust
use cpdb_rs::CpdbError;

match some_op() {
    Ok(value) => { /* ... */ }
    Err(CpdbError::NullPointer) => eprintln!("null pointer"),
    Err(CpdbError::NotFound(what)) => eprintln!("not found: {what}"),
    Err(CpdbError::JobFailed(msg)) => eprintln!("job failed: {msg}"),
    Err(e) => eprintln!("other: {e}"),
}
```

## Building the `ffi` feature on macOS

The `ffi` feature can be *compiled* on macOS but not linked without a
Linux D-Bus environment. Use `CPDB_NO_LINK=1` to skip link directives:

```bash
CPDB_NO_LINK=1 cargo build --lib --no-default-features --features ffi
```

The default `zbus-backend` build has no such caveat on macOS.

## Testing

```bash
# Unit tests — no live D-Bus required
cargo test --workspace

# Integration tests — require a running session bus and CPDB backends
cargo test --workspace -- --ignored
```

## Troubleshooting

### Default (`zbus-backend`)

- **`D-Bus connection failed` / `zbus: Address::from_env failed`** —
  Confirm a session bus is running (`echo $DBUS_SESSION_BUS_ADDRESS`)
  and that CPDB backend services (e.g. `cpdb-backend-cups`) are
  installed. In headless environments spin up an ephemeral bus with
  `dbus-launch --exit-with-session <command>`.

- **`No printers found`** — Verify printers are configured in the
  underlying spooler (CUPS, IPP, …) and the corresponding CPDB
  backend service is reachable over the session bus. A backend can
  take a moment to auto-activate; `CpdbClient` retries a few times,
  but a fully idle bus will still return an empty list.

- **Windows is not supported** — no CPDB backend daemon exists on
  Windows. Develop inside WSL Ubuntu; the repository on `/mnt/c/…` is
  reachable from WSL without copying.

### `ffi` feature only

- **`undefined symbol: cpdbGetVersion` / `libcpdb.so.3, may conflict with libcpdb.so.2`** —
  You have apt's older cpdb-libs **2.x** installed alongside the
  source-built **3.x**. The linker is picking the v2 library. Fix:

  ```bash
  sudo apt-get remove --purge libcpdb-dev libcpdb2t64
  sudo ldconfig
  cd ~/cpdb-libs && sudo make install   # reinstall headers v2 took with it
  cd /path/to/your/project && cargo clean && cargo build --features ffi
  ```

- **`fatal error: 'cpdb/cpdb.h' file not found`** — Headers are missing
  from `/usr/include/cpdb/`. Reinstall cpdb-libs from source (see
  [Prerequisites (for the `ffi` feature)](#prerequisites-for-the-ffi-feature)).

- **`Unable to find libclang` (bindgen)** — Install `libclang-dev` and
  `clang` (Debian/Ubuntu) or the equivalent on your distro.

- **`cpdb-libs not found`** — Install cpdb-libs 3.x so pkg-config can
  locate `cpdb.pc`. Override the discovery path with
  `CPDB_LIBS_PATH=<prefix>` when working against an uninstalled checkout.

- **Linker errors on a non-standard cpdb-libs prefix** — Set
  `PKG_CONFIG_PATH=<prefix>/lib/pkgconfig` so pkg-config can resolve
  `cpdb` and `cpdb-frontend` from your install.

- **`error: linker 'link.exe' not found` on Windows native** — The
  `ffi` feature does not support Windows targets. Develop inside WSL
  Ubuntu.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

1. Fork and clone.
2. `cargo build --workspace --all-targets` — the default
   (`zbus-backend`) path needs no C dependencies. If you also want to
   touch the `ffi` path, install cpdb-libs 3.x first (see
   [Prerequisites (for the `ffi` feature)](#prerequisites-for-the-ffi-feature))
   and add `--features ffi`.
3. Make changes, add tests.
4. Ensure the following all pass:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```
5. Open a pull request.

## License

MIT — see [LICENSE](LICENSE).

## Related projects

- [cpdb-libs](https://github.com/OpenPrinting/cpdb-libs) — the upstream C library. `cpdb-rs` speaks its D-Bus protocol directly on the default backend, and wraps its C API when built with `--features ffi`.
- [OpenPrinting](https://openprinting.org/)
- [CUPS](https://www.cups.org/)

## Support

- [Issues](https://github.com/OpenPrinting/cpdb-rs/issues)
- [Discussions](https://github.com/OpenPrinting/cpdb-rs/discussions)
- [API docs](https://docs.rs/cpdb-rs)

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
