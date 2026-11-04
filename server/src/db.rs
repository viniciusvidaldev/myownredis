use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use tokio::{
    sync::Notify,
    time::{self, Instant},
};

pub(crate) struct DbDropGuard {
    db: Db,
}

impl DbDropGuard {
    pub(crate) fn new() -> DbDropGuard {
        DbDropGuard { db: Db::new() }
    }

    pub(crate) fn db(&self) -> Db {
        self.db.clone()
    }
}

impl Drop for DbDropGuard {
    fn drop(&mut self) {
        self.db.shutdown_purge_task();
    }
}

#[derive(Clone)]
pub(crate) struct Db {
    shared: Arc<Shared>,
}

struct Shared {
    state: Mutex<State>,
    background_task: Notify,
}

struct Entry {
    value: Bytes,
    expires_at: Option<Instant>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Expiration {
    // Field order is the sort order: derived Ord compares when first, then key.
    when: Instant,
    key: String,
}

struct State {
    entries: HashMap<String, Entry>,
    expirations: BTreeSet<Expiration>,
    shutdown: bool,
}

impl State {
    fn next_expiration(&self) -> Option<Instant> {
        self.expirations.first().map(|e| e.when)
    }
}

impl Db {
    fn new() -> Db {
        let state = State {
            entries: HashMap::new(),
            expirations: BTreeSet::new(),
            shutdown: false,
        };
        let shared = Arc::new(Shared {
            state: Mutex::new(state),
            background_task: Notify::new(),
        });
        tokio::spawn(expurge_expired_tasks(shared.clone()));
        Db { shared }
    }

    pub(crate) fn get(&self, key: &str) -> Option<Bytes> {
        let state = self.shared.state.lock().unwrap();
        let entry = state.entries.get(key)?;
        if entry.expires_at.is_some_and(|when| when <= Instant::now()) {
            return None;
        };
        Some(entry.value.clone())
    }

    pub(crate) fn set(&self, key: String, value: Bytes, expire: Option<Duration>) {
        let mut state = self.shared.state.lock().unwrap();
        let expires_at = expire.map(|duration| Instant::now() + duration);

        // Must be computed before inserting the new expiration, or it would compare against itself.
        let notify = match expires_at {
            Some(when) => state.next_expiration().is_none_or(|next| when < next),
            None => false,
        };

        let prev = state
            .entries
            .insert(key.clone(), Entry { value, expires_at });

        // Drop the old expiration so the new expires_at replaces it.
        if let Some(when) = prev.and_then(|entry| entry.expires_at) {
            let exp = Expiration {
                when,
                key: key.clone(),
            };
            state.expirations.remove(&exp);
        }
        if let Some(when) = expires_at {
            state.expirations.insert(Expiration { when, key });
        }
        drop(state);

        if notify {
            self.shared.background_task.notify_one();
        }
    }

    fn shutdown_purge_task(&self) {
        let mut state = self.shared.state.lock().unwrap();
        state.shutdown = true;
        drop(state);
        self.shared.background_task.notify_one();
    }
}

impl Shared {
    fn is_shutdown(&self) -> bool {
        self.state.lock().unwrap().shutdown
    }

    fn purge_expired_keys(&self) -> Option<Instant> {
        let mut state = self.state.lock().unwrap();
        let now = Instant::now();

        loop {
            let exp = state.expirations.first()?;

            if exp.when > now {
                return Some(exp.when);
            }

            let exp = state.expirations.pop_first().unwrap();
            state.entries.remove(&exp.key);
        }
    }
}

async fn expurge_expired_tasks(shared: Arc<Shared>) {
    while !shared.is_shutdown() {
        match shared.purge_expired_keys() {
            Some(when) => {
                // set() notifies when a sooner expiration arrives, so we wake early to reschedule.
                tokio::select! {
                    _ = time::sleep_until(when) => {},
                    _ = shared.background_task.notified() => {},
                }
            }
            // No deadline to sleep on, only a set() with a TTL can give us one.
            None => shared.background_task.notified().await,
        }
    }
}
