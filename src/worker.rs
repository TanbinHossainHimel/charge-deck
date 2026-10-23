//! Background thread: runs every provider, stores results, wakes the window.
//!
//! The window never talks to devices. It only reads `Shared` and sends a
//! "refresh now" message on a channel.

use crate::clock::now_hhmm;
use crate::model::Deck;
use crate::providers::{registry, run_guarded};
use hidapi::HidApi;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Gentle polling (rule 8): once every 2 minutes, plus manual refresh.
pub const REFRESH_EVERY: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct Shared {
    pub deck: Deck,
    pub last_updated: Option<String>,
    pub scanning: bool,
}

pub type SharedState = Arc<Mutex<Shared>>;

/// Start the worker. `wake_ui` is called after every update (the window passes
/// `ctx.request_repaint()`). Returns the sender used by the Refresh button.
pub fn spawn(shared: SharedState, wake_ui: impl Fn() + Send + 'static) -> Sender<()> {
    let (tx, rx) = channel::<()>();
    thread::spawn(move || {
        // HidApi is not Sync and providers are not Send, so both live
        // on this thread only.
        let mut hid: Option<HidApi> = None;
        let mut providers = registry();
        loop {
            lock(&shared).scanning = true;
            wake_ui();

            match hid.as_mut() {
                Some(api) => {
                    let _ = api.refresh_devices();
                }
                None => hid = HidApi::new().ok(),
            }
            if let Some(api) = hid.as_ref() {
                for p in providers.iter_mut() {
                    let found = run_guarded(p.as_mut(), api);
                    let now = now_hhmm();
                    // Show each provider's results as soon as it finishes.
                    lock(&shared).deck.apply(p.name(), found, &now);
                    wake_ui();
                }
            }

            {
                let mut s = lock(&shared);
                s.scanning = false;
                s.last_updated = Some(now_hhmm());
            }
            wake_ui();

            match rx.recv_timeout(REFRESH_EVERY) {
                Ok(()) => while rx.try_recv().is_ok() {}, // collapse repeated clicks
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return, // window closed
            }
        }
    });
    tx
}

/// Lock even if another thread panicked while holding the lock.
fn lock(shared: &SharedState) -> std::sync::MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}
