use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::Instant;

/// All possible states a tab can be in
#[derive(Debug, Clone, PartialEq)]
pub enum TabState {
    /// Just created, not yet loaded
    New,
    /// Currently loading a page
    Loading,
    /// Loaded and currently visible to user
    Active,
    /// Loaded but in background (not visible)
    Background,
    /// Suspended — renderer destroyed, just URL + title kept
    Hibernated,
    /// Renderer crashed
    Crashed,
}

impl TabState {
    /// Can this tab be hibernated right now?
    pub fn is_hibernatable(&self) -> bool {
        matches!(self, TabState::Background)
    }

    /// Is this tab currently consuming renderer RAM?
    pub fn has_renderer(&self) -> bool {
        matches!(self, TabState::Loading | TabState::Active | TabState::Background)
    }
}

/// A single browser tab
#[derive(Debug, Clone)]
pub struct Tab {
    /// Unique ID for this tab
    pub id: u32,
    /// Current URL
    pub url: String,
    /// Page title (from <title> tag)
    pub title: String,
    /// Current state
    pub state: TabState,
    /// Approximate RAM usage in KB (0 if hibernated)
    pub memory_kb: u64,
    /// When the user last had this tab active
    pub last_active: Instant,
}

impl Tab {
    pub fn new(id: u32, url: &str) -> Self {
        Self {
            id,
            url: url.to_string(),
            title: "New Tab".to_string(),
            state: TabState::New,
            memory_kb: 0,
            last_active: Instant::now(),
        }
    }

    /// How long since this tab was last active (in seconds)
    pub fn idle_secs(&self) -> u64 {
        self.last_active.elapsed().as_secs()
    }

    /// Should this tab be auto-hibernated?
    pub fn should_hibernate(&self, idle_threshold_secs: u64) -> bool {
        self.state.is_hibernatable()
            && self.idle_secs() >= idle_threshold_secs
    }
}

/// Thread-safe store for all open tabs
pub struct TabStore {
    tabs: Arc<RwLock<HashMap<u32, Tab>>>,
    /// Which tab is currently active
    active_id: Arc<RwLock<u32>>,
    /// Counter for generating unique tab IDs
    next_id: Arc<RwLock<u32>>,
}

impl TabStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tabs: Arc::new(RwLock::new(HashMap::new())),
            active_id: Arc::new(RwLock::new(0)),
            next_id: Arc::new(RwLock::new(1)),
        })
    }

    /// Open a new tab with the given URL
    /// Returns the new tab's ID
    pub fn open(&self, url: &str) -> u32 {
        let mut next = self.next_id.write().unwrap();
        let id = *next;
        *next += 1;
        drop(next); // release lock before acquiring tabs lock

        let tab = Tab::new(id, url);
        self.tabs.write().unwrap().insert(id, tab);

        log::info!("Opened tab {}: {}", id, url);
        id
    }

    /// Close a tab by ID
    pub fn close(&self, id: u32) {
        self.tabs.write().unwrap().remove(&id);
        log::info!("Closed tab {}", id);
    }

    /// Set which tab is currently active
    pub fn set_active(&self, id: u32) {
        // Mark previous active tab as background
        let current = *self.active_id.read().unwrap();
        if current != 0 && current != id {
            let mut tabs = self.tabs.write().unwrap();
            if let Some(tab) = tabs.get_mut(&current) {
                if tab.state == TabState::Active {
                    tab.state = TabState::Background;
                    log::info!("Tab {} moved to background", current);
                }
            }
        }

        // Mark new tab as active
        let mut tabs = self.tabs.write().unwrap();
        if let Some(tab) = tabs.get_mut(&id) {
            tab.state = TabState::Active;
            tab.last_active = Instant::now();
            log::info!("Tab {} is now active", id);
        }

        *self.active_id.write().unwrap() = id;
    }

    /// Get the currently active tab ID
    pub fn active_id(&self) -> u32 {
        *self.active_id.read().unwrap()
    }

    /// Get a snapshot of all tabs (sorted by ID)
    pub fn get_all(&self) -> Vec<Tab> {
        let mut tabs: Vec<Tab> = self.tabs
            .read().unwrap()
            .values()
            .cloned()
            .collect();
        tabs.sort_by_key(|t| t.id);
        tabs
    }

    /// Get total RAM used across all active tabs
    pub fn total_memory_kb(&self) -> u64 {
        self.tabs.read().unwrap()
            .values()
            .map(|t| t.memory_kb)
            .sum()
    }

    /// Update a tab's title
    pub fn set_title(&self, id: u32, title: &str) {
        let mut tabs = self.tabs.write().unwrap();
        if let Some(tab) = tabs.get_mut(&id) {
            tab.title = title.to_string();
        }
    }

    /// Update a tab's URL
    pub fn set_url(&self, id: u32, url: &str) {
        let mut tabs = self.tabs.write().unwrap();
        if let Some(tab) = tabs.get_mut(&id) {
            tab.url = url.to_string();
        }
    }

    /// Update a tab's state
    pub fn set_state(&self, id: u32, state: TabState) {
        let mut tabs = self.tabs.write().unwrap();
        if let Some(tab) = tabs.get_mut(&id) {
            log::info!("Tab {} state: {:?} -> {:?}", id, tab.state, state);
            tab.state = state;
        }
    }

    /// Get tabs eligible for hibernation
    pub fn hibernatable_tabs(&self, idle_threshold_secs: u64) -> Vec<Tab> {
        self.tabs.read().unwrap()
            .values()
            .filter(|t| t.should_hibernate(idle_threshold_secs))
            .cloned()
            .collect()
    }

    /// How many tabs are currently alive (not hibernated)
    pub fn active_count(&self) -> usize {
        self.tabs.read().unwrap()
            .values()
            .filter(|t| t.state.has_renderer())
            .count()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_close() {
        let store = TabStore::new();
        let id = store.open("https://google.com");
        assert_eq!(store.get_all().len(), 1);
        store.close(id);
        assert_eq!(store.get_all().len(), 0);
    }

    #[test]
    fn test_active_state() {
        let store = TabStore::new();
        let id1 = store.open("https://google.com");
        let id2 = store.open("https://github.com");

        store.set_active(id1);
        assert_eq!(store.active_id(), id1);

        store.set_active(id2);
        assert_eq!(store.active_id(), id2);

        // id1 should now be in background
        let tabs = store.get_all();
        let tab1 = tabs.iter().find(|t| t.id == id1).unwrap();
        assert_eq!(tab1.state, TabState::Background);
    }

    #[test]
    fn test_hibernation_eligibility() {
        let store = TabStore::new();
        let id = store.open("https://google.com");

        // New tab — not hibernatable
        let tabs = store.get_all();
        let tab = tabs.iter().find(|t| t.id == id).unwrap();
        assert!(!tab.should_hibernate(300));

        // Set to background
        store.set_state(id, TabState::Background);

        // Background but just became background — not idle enough
        let tabs = store.get_all();
        let tab = tabs.iter().find(|t| t.id == id).unwrap();
        assert!(!tab.should_hibernate(300)); // 300s not elapsed

        // Would hibernate immediately with 0 threshold
        assert!(tab.should_hibernate(0));
    }

    #[test]
    fn test_memory_total() {
        let store = TabStore::new();
        let id1 = store.open("https://google.com");
        let id2 = store.open("https://github.com");

        {
            let mut tabs = store.tabs.write().unwrap();
            tabs.get_mut(&id1).unwrap().memory_kb = 60_000; // 60MB
            tabs.get_mut(&id2).unwrap().memory_kb = 80_000; // 80MB
        }

        assert_eq!(store.total_memory_kb(), 140_000); // 140MB total
    }
}