//! Async CPDB client.
//!
//! [`CpdbClient`] discovers all CPDB backends on the D-Bus session bus
//! and provides methods for printer enumeration, capability querying,
//! and job submission.
//!
//! # Example
//!
//! ```rust,no_run
//! use cpdb_rs::CpdbClient;
//!
//! # async fn example() -> cpdb_rs::Result<()> {
//! let client = CpdbClient::new().await?;
//! let printers = client.get_all_printers().await?;
//! for p in &printers {
//!     println!("{} [{}]", p.name, p.id);
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{Stream, StreamExt, stream::SelectAll};
use zbus::zvariant::OwnedFd;

use crate::error::{CpdbError, Result};
use crate::events::{DiscoveryEvent, PrinterSnapshot};
use crate::media::MediaCollection;
use crate::options::OptionsCollection;
use crate::proxy::PrintBackendProxy;
use crate::types::PrinterState;

const AUTO_EXIT_TIMEOUT: Duration = Duration::from_secs(30);
const RETRY_INTERVAL_MS: Duration = Duration::from_millis(200);

/// Retries a D-Bus proxy call once if the backend returns `UnknownMethod`.
///
/// This handles the activation race where systemd has started the backend
/// process but it hasn't registered its methods yet. The retry is safe for
/// all call sites because `UnknownMethod` guarantees no side-effect occurred
/// (no job was created, no state was mutated). Arguments are evaluated twice
/// on retry, which is fine since they are all side-effect-free references.
macro_rules! retry_dbus {
    ($proxy:expr, $method:ident($($arg:expr),*)) => {{
        let __proxy = &($proxy);
        let mut __result = __proxy.$method($($arg),*).await;
        if let Err(zbus::Error::MethodError(ref __n, _, _)) = __result {
            if __n.as_str() == "org.freedesktop.DBus.Error.UnknownMethod" {
                tokio::time::sleep(RETRY_INTERVAL_MS).await;
                __result = __proxy.$method($($arg),*).await;
            }
        }
        __result
    }};
}

/// A connected CPDB client managing proxies to all discovered print backends.
///
/// Created via [`CpdbClient::new()`]. The client is [`Clone`]-able - cloning
/// shares the underlying D-Bus connection.
///
/// # Usage
///
/// ```rust,no_run
/// # use cpdb_rs::CpdbClient;
/// # async fn example() -> cpdb_rs::Result<()> {
/// let client = CpdbClient::new().await?;
///
/// // Initial population
/// let printers = client.get_all_printers().await?;
///
/// // Fetch capabilities when the user selects a printer
/// let (options, media) = client.get_printer_details(&printers[0].id, "CUPS").await?;
///
/// // Submit a print job
/// let settings = [("copies", "1"), ("media", "iso_a4_210x297mm")];
/// let (job_id, fd) = client.print_fd(&printers[0].id, "CUPS", &settings, "My Doc").await?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct CpdbClient {
    backends: Vec<BackendHandle>,
}

/// Internal handle for a single backend (e.g. CUPS).
#[derive(Clone)]
struct BackendHandle {
    /// Full D-Bus service name, e.g. `"org.openprinting.Backend.CUPS"`.
    service_name: String,
    /// The zbus-generated proxy for this backend's PrintBackend interface.
    proxy: PrintBackendProxy<'static>,
}

impl CpdbClient {
    /// Connect to the D-Bus session bus and discover all CPDB backends.
    ///
    /// 1. Opens a session bus connection.
    /// 2. Calls `ListActivatableNames` to find `org.openprinting.Backend.*` services.
    /// 3. Creates a [`PrintBackendProxy`] for each discovered backend.
    ///
    /// Backends that fail to connect are logged and skipped.
    ///
    /// # Errors
    ///
    /// Returns [`CpdbError::DbusError`] if the session bus itself is unavailable.
    pub async fn new() -> Result<Self> {
        let connection = zbus::Connection::session().await.map_err(CpdbError::from)?;

        let dbus = zbus::fdo::DBusProxy::new(&connection)
            .await
            .map_err(CpdbError::from)?;
        let names = dbus
            .list_activatable_names()
            .await
            .map_err(CpdbError::from)?;

        let backend_names: Vec<String> = names
            .iter()
            .filter(|n| n.starts_with("org.openprinting.Backend."))
            .map(|n| n.to_string())
            .collect();

        let mut backends = Vec::new();
        for name in &backend_names {
            let bus_name = match zbus::names::BusName::try_from(name.clone()) {
                Ok(n) => n,
                Err(_) => continue,
            };
            match PrintBackendProxy::builder(&connection)
                .destination(bus_name)?
                .path("/")?
                .build()
                .await
            {
                Ok(proxy) => {
                    backends.push(BackendHandle {
                        service_name: name.clone(),
                        proxy,
                    });
                }
                Err(e) => {
                    log::warn!("cpdb-rs: skipping backend {}: {}", name, e);
                }
            }
        }

        Ok(Self { backends })
    }

    /// Returns the number of connected backends.
    pub fn backend_count(&self) -> usize {
        self.backends.len()
    }

    /// Fetches all known printers from all connected backends.
    ///
    /// This is the **initial population** method - equivalent to what the C
    /// library's `fetchPrinterListFromBackend()` does. It calls `GetAllPrinters`
    /// on each backend and unpacks the variant-wrapped printer data into
    /// [`PrinterSnapshot`]s.
    ///
    /// Use this to populate the printer list.
    /// Use [`discovery_stream()`](Self::discovery_stream) for live updates after that.
    ///
    /// # Errors
    ///
    /// Returns errors if a D-Bus call fails. Backends that fail individually
    /// are skipped, and printers from working backends are still returned.
    pub async fn get_all_printers(&self) -> Result<Vec<PrinterSnapshot>> {
        let mut printers = Vec::new();

        for bh in &self.backends {
            let result = retry_dbus!(bh.proxy, get_all_printers());

            let (_count, raw_printers) = match result {
                Ok(v) => v,
                Err(e) => {
                    log::error!(
                        "cpdb-rs: error fetching printers from {}: {}",
                        bh.service_name,
                        e
                    );
                    continue;
                }
            };

            for raw in raw_printers {
                printers.push(PrinterSnapshot {
                    id: raw.0,
                    name: raw.1,
                    info: raw.2,
                    location: raw.3,
                    make_model: raw.4,
                    accepting_jobs: raw.5,
                    state: PrinterState::from(raw.6),
                    backend: raw.7,
                });
            }
        }

        Ok(printers)
    }

    /// Like [`get_all_printers()`](Self::get_all_printers) but returns only
    /// printers matching the current filter state.
    ///
    /// Call [`show_remote_printers(false)`](Self::show_remote_printers) or
    /// [`show_temporary_printers(false)`](Self::show_temporary_printers)
    /// first to set the filter, then call this to get the filtered list.
    pub async fn get_filtered_printers(&self) -> Result<Vec<PrinterSnapshot>> {
        let mut printers = Vec::new();

        for bh in &self.backends {
            let result = retry_dbus!(bh.proxy, get_filtered_printer_list());

            let (_count, raw_printers) = match result {
                Ok(v) => v,
                Err(e) => {
                    log::error!(
                        "cpdb-rs: error fetching filtered printers from {}: {}",
                        bh.service_name,
                        e
                    );
                    continue;
                }
            };

            for raw in raw_printers {
                printers.push(PrinterSnapshot {
                    id: raw.0,
                    name: raw.1,
                    info: raw.2,
                    location: raw.3,
                    make_model: raw.4,
                    accepting_jobs: raw.5,
                    state: PrinterState::from(raw.6),
                    backend: raw.7,
                });
            }
        }

        Ok(printers)
    }

    /// Returns a merged stream of [`DiscoveryEvent`]s from all backends.
    ///
    /// The stream emits events as printers are added, removed, or change
    /// state. After subscribing to signals, it calls `doListing(true)` on
    /// each backend to trigger initial `PrinterAdded` emissions.
    ///
    /// A background task is automatically spawned to ping the backends
    /// periodically, preventing them from timing out. When this stream is
    /// dropped, the keep-alive task is cancelled.
    ///
    /// # Errors
    ///
    /// Returns [`CpdbError::DbusError`] if subscribing to D-Bus signals fails.
    pub async fn discovery_stream(&self) -> Result<DiscoveryStream> {
        let mut all: SelectAll<futures_util::stream::BoxStream<'static, DiscoveryEvent>> =
            SelectAll::new();

        for bh in &self.backends {
            // Subscribe to PrinterAdded signals
            let added = bh
                .proxy
                .receive_printer_added()
                .await
                .map_err(CpdbError::from)?;
            all.push(
                added
                    .filter_map(|sig| async move {
                        match sig.args() {
                            Ok(a) => Some(DiscoveryEvent::PrinterAdded(PrinterSnapshot {
                                id: a.printer_id.to_string(),
                                name: a.printer_name.to_string(),
                                info: a.printer_info.to_string(),
                                location: a.printer_location.to_string(),
                                make_model: a.printer_make_and_model.to_string(),
                                accepting_jobs: a.printer_is_accepting_jobs,
                                state: PrinterState::from(a.printer_state),
                                backend: a.backend_name.to_string(),
                            })),
                            Err(e) => {
                                log::debug!("cpdb-rs: failed to parse PrinterAdded signal: {}", e);
                                None
                            }
                        }
                    })
                    .boxed(),
            );

            // Subscribe to PrinterRemoved signals
            let removed = bh
                .proxy
                .receive_printer_removed()
                .await
                .map_err(CpdbError::from)?;
            all.push(
                removed
                    .filter_map(|sig| async move {
                        match sig.args() {
                            Ok(a) => Some(DiscoveryEvent::PrinterRemoved {
                                id: a.printer_id.to_string(),
                                backend: a.backend_name.to_string(),
                            }),
                            Err(e) => {
                                log::debug!(
                                    "cpdb-rs: failed to parse PrinterRemoved signal: {}",
                                    e
                                );
                                None
                            }
                        }
                    })
                    .boxed(),
            );

            // Subscribe to PrinterStateChanged signals
            let changed = bh
                .proxy
                .receive_printer_state_changed()
                .await
                .map_err(CpdbError::from)?;
            all.push(
                changed
                    .filter_map(|sig| async move {
                        match sig.args() {
                            Ok(a) => Some(DiscoveryEvent::PrinterStateChanged {
                                id: a.printer_id.to_string(),
                                backend: a.backend_name.to_string(),
                                state: PrinterState::from(a.printer_state),
                                accepting_jobs: a.printer_is_accepting_jobs,
                            }),
                            Err(e) => {
                                log::debug!(
                                    "cpdb-rs: failed to parse PrinterStateChanged signal: {}",
                                    e
                                );
                                None
                            }
                        }
                    })
                    .boxed(),
            );
        }

        for bh in &self.backends {
            if let Err(e) = bh.proxy.do_listing(true).await {
                log::warn!(
                    "cpdb-rs: do_listing failed for {}, discovery stream may be empty: {}",
                    bh.service_name,
                    e
                );
            }
        }

        let client = self.clone();
        let keep_alive_task = tokio::spawn(async move {
            let ping_interval = std::time::Duration::from_secs(AUTO_EXIT_TIMEOUT.as_secs() / 2);
            loop {
                tokio::time::sleep(ping_interval).await;
                client.keep_alive_all().await;
            }
        });

        Ok(DiscoveryStream {
            inner: all,
            keep_alive_task,
        })
    }

    /// Keeps all backends alive to prevent them from auto-exiting.
    ///
    /// CPDB backends automatically exit after a period of inactivity (typically 30 seconds).
    /// If you are listening to a `discovery_stream()`, you should call this method
    /// periodically (e.g. every 10 seconds) to ensure the backends stay running
    /// and continue emitting discovery signals.
    pub async fn keep_alive_all(&self) {
        for bh in &self.backends {
            let _ = bh.proxy.keep_alive().await;
        }
    }

    /// Fetches all options and media for a printer in a single D-Bus call.
    ///
    /// This calls the backend's `GetAllOptions` method, which returns both
    /// the printer's capabilities (duplex, color mode, etc.) and its
    /// supported paper sizes with margin information.
    ///
    /// # Arguments
    ///
    /// * `printer_id` - The printer's unique ID (from [`PrinterSnapshot::id`]).
    /// * `backend` - The backend name, e.g. `"CUPS"` or the full service
    ///   name `"org.openprinting.Backend.CUPS"`.
    ///
    /// # Errors
    ///
    /// * [`CpdbError::BackendError`] if no backend matches `backend`.
    /// * [`CpdbError::DbusError`] if the D-Bus call fails.
    pub async fn get_printer_details(
        &self,
        printer_id: &str,
        backend: &str,
    ) -> Result<(OptionsCollection, MediaCollection)> {
        let proxy = self.proxy_for(backend)?;
        let (_n_opts, raw_opts, _n_media, raw_media) =
            retry_dbus!(proxy, get_all_options(printer_id)).map_err(CpdbError::from)?;
        Ok((
            OptionsCollection::from_dbus(raw_opts),
            MediaCollection::from_dbus(raw_media),
        ))
    }

    /// Fetches localized labels for a printer's options and choices.
    ///
    /// Returns a map of internal name -> human-readable label, e.g.
    /// `{"sides" -> "Two-Sided", "one-sided" -> "Off", ...}`.
    ///
    /// # Arguments
    ///
    /// * `printer_id` - The printer's unique ID.
    /// * `backend` - The backend name.
    /// * `locale` - A POSIX locale string, e.g. `"en_US"` or `"de_DE"`.
    pub async fn get_translations(
        &self,
        printer_id: &str,
        backend: &str,
        locale: &str,
    ) -> Result<HashMap<String, String>> {
        let proxy = self.proxy_for(backend)?;
        retry_dbus!(proxy, get_all_translations(printer_id, locale)).map_err(CpdbError::from)
    }

    /// Returns the default printer ID for a specific backend.
    pub async fn get_default_printer(&self, backend: &str) -> Result<String> {
        let proxy = self.proxy_for(backend)?;
        retry_dbus!(proxy, get_default_printer()).map_err(CpdbError::from)
    }

    /// Submits a print job and returns a writable file descriptor.
    ///
    /// The backend creates a CUPS job and returns the write end of a
    /// socketpair. The caller writes the document data into `fd` and
    /// closes it when done - the backend reads from the other end and
    /// forwards it to the print system.
    ///
    /// # Arguments
    ///
    /// * `printer_id` - The printer's unique ID.
    /// * `backend` - The backend name.
    /// * `settings` - Print settings as key-value pairs, e.g.
    ///   `[("copies", "2"), ("media", "iso_a4_210x297mm")]`.
    /// * `title` - The job title shown in the print queue.
    ///
    /// # Returns
    ///
    /// A tuple of `(job_id, fd)` where `job_id` is the CUPS job ID
    /// string and `fd` is the writable end of the socketpair.
    pub async fn print_fd(
        &self,
        printer_id: &str,
        backend: &str,
        settings: &[(&str, &str)],
        title: &str,
    ) -> Result<(String, OwnedFd)> {
        let proxy = self.proxy_for(backend)?;
        let (job_id, fd) = retry_dbus!(
            proxy,
            print_fd(printer_id, settings.len() as i32, settings, title)
        )
        .map_err(CpdbError::from)?;
        Ok((job_id, fd))
    }

    /// Submits a print job and returns a Unix domain socket path to write the document to.
    ///
    /// The caller must connect to the returned socket path and write the document
    /// data, closing the stream when finished.
    pub async fn print_socket(
        &self,
        printer_id: &str,
        backend: &str,
        settings: &[(&str, &str)],
        title: &str,
    ) -> Result<(String, String)> {
        let proxy = self.proxy_for(backend)?;
        let (job_id, socket_path) = retry_dbus!(
            proxy,
            print_socket(printer_id, settings.len() as i32, settings, title)
        )
        .map_err(CpdbError::from)?;
        Ok((job_id, socket_path))
    }

    /// Sets the visibility of remote printers on all connected backends.
    ///
    /// When `visible` is `false`, printers discovered via DNS-SD / mDNS
    /// on remote hosts are hidden from discovery signals.
    pub async fn show_remote_printers(&self, visible: bool) {
        for b in &self.backends {
            let _ = b.proxy.show_remote_printers(visible).await;
        }
    }

    /// Sets the visibility of temporary (auto-discovered) printers on
    /// all connected backends.
    pub async fn show_temporary_printers(&self, visible: bool) {
        for b in &self.backends {
            let _ = b.proxy.show_temporary_printers(visible).await;
        }
    }

    /// Finds the proxy for a backend by name.
    ///
    /// Accepts either a short name (`"CUPS"`) or a full D-Bus service
    /// name (`"org.openprinting.Backend.CUPS"`).
    fn proxy_for(&self, backend: &str) -> Result<&PrintBackendProxy<'static>> {
        self.backends
            .iter()
            .find(|b| {
                if b.service_name == backend {
                    return true;
                }
                if let Some(idx) = b.service_name.rfind('.') {
                    &b.service_name[idx + 1..] == backend
                } else {
                    false
                }
            })
            .map(|b| &b.proxy)
            .ok_or_else(|| {
                CpdbError::BackendError(format!("No backend found matching '{}'", backend))
            })
    }
}

/// A stream of live discovery events that keeps connected backends alive.
pub struct DiscoveryStream {
    inner: SelectAll<futures_util::stream::BoxStream<'static, DiscoveryEvent>>,
    keep_alive_task: tokio::task::JoinHandle<()>,
}

impl Stream for DiscoveryStream {
    type Item = DiscoveryEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Drop for DiscoveryStream {
    fn drop(&mut self) {
        self.keep_alive_task.abort();
    }
}
