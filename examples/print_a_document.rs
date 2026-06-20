//! Example: Submit a print job via print_fd() with fallback to print_socket().
//!
//! Accepts printer ID, file path, and optional backend name via command-line arguments.

use cpdb_rs::{CpdbClient, CpdbError};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: cargo run --example print_a_document <printer_id> <path_to_file> [backend]"
        );
        let client = CpdbClient::new().await?;
        let _ = client.get_all_printers().await; // Warm up
        let printers = client.get_all_printers().await?;
        eprintln!("\nAvailable printers:");
        for p in &printers {
            eprintln!(
                "  Name: {:<20} ID: {:<20} Backend: {}",
                p.name, p.id, p.backend
            );
        }
        return Ok(());
    }

    let printer_id = &args[1];
    let file_path = &args[2];
    let backend = if args.len() >= 4 { &args[3] } else { "CUPS" };

    println!("Reading file: {}", file_path);
    let postscript = match std::fs::read(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file '{}': {}", file_path, e);
            return Ok(());
        }
    };

    let client = CpdbClient::new().await?;
    let _ = client.get_all_printers().await; // Warm up
    let printers = client.get_all_printers().await?;
    let target = printers
        .iter()
        .find(|p| p.id.as_str() == printer_id && p.backend.as_str() == backend);
    match target {
        Some(p) => println!(
            "Target: {} [state={}, accepting={}]",
            p.name, p.state, p.accepting_jobs
        ),
        None => {
            eprintln!(
                "Warning: Printer '{}' not found in active list for backend '{}'. Trying anyway...",
                printer_id, backend
            );
        }
    }

    let settings = [("copies", "2"), ("media", "iso_a4_210x297mm")];
    let title = "cpdb-rs CLI print test";

    println!("\nSubmitting job to '{}' via {}...", printer_id, backend);

    match client.print_fd(printer_id, backend, &settings, title).await {
        Ok((job_id, fd)) => {
            println!("printFd SUCCESS: Job ID: {}", job_id);

            // Convert the zbus OwnedFd into a std OwnedFd, then into a File to prevent double-drop
            let std_fd: std::os::fd::OwnedFd = fd.into();
            let mut file = std::fs::File::from(std_fd);
            use std::io::Write;
            file.write_all(&postscript).expect("Failed to write to FD");
            drop(file); // Signals EOF

            println!("Document written to FD and stream closed.");
        }
        Err(CpdbError::DbusError(zbus::Error::MethodError(name, _, _)))
            if name.as_str() == "org.freedesktop.DBus.Error.UnknownMethod" =>
        {
            println!("printFd not supported by backend. Falling back to printSocket...");

            let (job_id, socket_path) = client
                .print_socket(printer_id, backend, &settings, title)
                .await?;

            println!(
                "printSocket SUCCESS: Job ID: {}, Socket: {}",
                job_id, socket_path
            );

            let mut stream = UnixStream::connect(&socket_path)
                .await
                .expect("Failed to connect to print socket");

            stream
                .write_all(&postscript)
                .await
                .expect("Failed to write to socket");

            stream.shutdown().await.unwrap(); // Signals EOF
            println!("Document written to socket and stream closed.");
        }
        Err(e) => {
            return Err(e);
        }
    }

    println!("\nCheck the job with: lpstat -o");

    Ok(())
}
