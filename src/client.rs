//! The KMS makes outbound connections to the validator, and is technically a
//! client, however once connected it accepts incoming RPCs, and otherwise
//! acts as a service.
//!
//! To dance around the fact the KMS isn't actually a service, we refer to it
//! as a "Key Management System".

use std::panic;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use session::Session;

/// How long to wait after a crash before respawning (in seconds)
pub const RESPAWN_DELAY: u64 = 5;

/// Client connections: wraps a thread which makes a connection to a particular
/// validator node and then receives RPCs.
///
/// The `Client` type does not deal with network I/O, that is handled inside of
/// the `Session`. Instead, the `Client` type manages threading and respawning
/// sessions in the event of errors.
pub struct Client {
    /// Handle to the client thread
    handle: JoinHandle<()>,
}

impl Client {
    /// Spawn a new client, returning a handle so it can be joined
    pub fn spawn<'k>(addr: String, port: u16) -> Self {
        Self {
            handle: thread::spawn(move || client_loop(&addr, port)),
        }
    }

    /// Wait for a running client to finish
    pub fn join(self) {
        self.handle.join().unwrap();
    }
}

/// Main loop for all clients. Handles reconnecting in the event of an error
fn client_loop(addr: &str, port: u16) {
    loop {
        match panic::catch_unwind(|| Session::new(addr, port)?.handle_requests()) {
            Ok(result) => match result {
                Ok(_) => {
                    info!("[{}:{}] session closed gracefully", addr, port);
                    return;
                }
                Err(e) => error!("[{}:{}] {}", addr, port, e),
            },
            Err(val) => {
                if let Some(e) = val.downcast_ref::<String>() {
                    error!("[{}:{}] client panic! {}", addr, port, e);
                } else if let Some(e) = val.downcast_ref::<&str>() {
                    error!("[{}:{}] client panic! {}", addr, port, e);
                } else {
                    error!("[{}:{}] client panic! (unknown cause)", addr, port);
                }
            }
        }

        // TODO: exponential backoff?
        thread::sleep(Duration::from_secs(RESPAWN_DELAY))
    }
}
