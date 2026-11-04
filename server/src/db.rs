use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
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

impl Entry {
    fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|when| when <= now)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Expiration {
    // Field order is the sort order: derived Ord compares when first, then key.
    when: Instant,
    key: Bytes,
}

struct State {
    entries: HashMap<Bytes, Entry>,
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

    pub(crate) fn get(&self, key: &[u8]) -> Option<Bytes> {
        let state = self.shared.state.lock().unwrap();
        let entry = state.entries.get(key)?;
        if entry.is_expired(Instant::now()) {
            return None;
        }
        Some(entry.value.clone())
    }

    pub(crate) fn set(&self, key: Bytes, value: Bytes, expires_at: Option<Instant>) {
        let mut state = self.shared.state.lock().unwrap();

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

    pub(crate) fn exists(&self, keys: &[Bytes]) -> usize {
        let state = self.shared.state.lock().unwrap();
        let now = Instant::now();
        let mut amount = 0;
        for key in keys {
            let Some(entry) = state.entries.get(key) else {
                continue;
            };
            if !entry.is_expired(now) {
                amount += 1;
            }
        }
        amount
    }

    pub(crate) fn del(&self, keys: &[Bytes]) -> usize {
        let mut state = self.shared.state.lock().unwrap();
        let now = Instant::now();
        let mut deleted = 0;
        for key in keys {
            let Some(entry) = state.entries.remove(key) else {
                continue;
            };

            if let Some(when) = entry.expires_at {
                let exp = Expiration {
                    when,
                    key: key.clone(),
                };
                state.expirations.remove(&exp);
            }

            if !entry.is_expired(now) {
                deleted += 1;
            }
        }
        deleted
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
