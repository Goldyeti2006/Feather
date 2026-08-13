use std::sync::Arc;
use crate::shell::tabs::{TabStore, TabState};

pub struct HibernationManager {
    store: Arc<TabStore>,
    /// Seconds a tab must be idle before hibernation
    idle_threshold_secs: u64,
}

impl HibernationManager {
    pub fn new(store: Arc<TabStore>) -> Self {
        Self {
            store,
            idle_threshold_secs: 300, // 5 minutes default
        }
    }

    pub fn with_threshold(mut self, secs: u64) -> Self {
        self.idle_threshold_secs = secs;
        self
    }

    /// Hibernate a specific tab by ID
    /// Returns true if hibernation happened
    pub fn hibernate(&self, tab_id: u32) -> bool {
        let tabs = self.store.tabs.read().unwrap();
        let tab = match tabs.get(&tab_id) {
            Some(t) => t.clone(),
            None => return false,
        };
        drop(tabs);

        // Only hibernate background tabs
        if tab.state != TabState::Background {
            log::warn!(
                "Cannot hibernate tab {} — state is {:?}",
                tab_id, tab.state
            );
            return false;
        }

        // In a full implementation this is where you'd call:
        // webview.destroy_renderer(tab_id)
        // For now we mark it in the store
        self.store.set_state(tab_id, TabState::Hibernated);

        log::info!(
            "Hibernated tab {}: {} (was idle {}s)",
            tab_id, tab.url, tab.idle_secs()
        );

        true
    }

    /// Wake a hibernated tab
    /// Returns the URL to reload, or None if tab not found
    pub fn wake(&self, tab_id: u32) -> Option<String> {
        let tabs = self.store.tabs.read().unwrap();
        let tab = match tabs.get(&tab_id) {
            Some(t) => t.clone(),
            None => return None,
        };
        drop(tabs);

        if tab.state != TabState::Hibernated {
            log::warn!(
                "Cannot wake tab {} — state is {:?}",
                tab_id, tab.state
            );
            return None;
        }

        self.store.set_state(tab_id, TabState::Loading);
        log::info!("Waking tab {}: {}", tab_id, tab.url);

        Some(tab.url)
    }

    /// Run one GC pass — hibernate eligible background tabs
    /// Returns how many tabs were hibernated
    pub fn run_gc_pass(&self) -> usize {
        let candidates = self.store
            .hibernatable_tabs(self.idle_threshold_secs);

        if candidates.is_empty() {
            return 0;
        }

        log::info!(
            "GC pass: {} tabs eligible for hibernation",
            candidates.len()
        );

        let mut count = 0;
        for tab in candidates {
            if self.hibernate(tab.id) {
                count += 1;
            }
        }

        count
    }

    /// Run GC pass triggered by memory pressure
    /// More aggressive — hibernates after shorter idle time
    pub fn run_pressure_gc(&self) -> usize {
        let pressure_threshold = self.idle_threshold_secs / 5;
        let candidates = self.store
            .hibernatable_tabs(pressure_threshold);

        log::warn!(
            "Memory pressure GC: {} tabs eligible",
            candidates.len()
        );

        let mut count = 0;
        for tab in candidates {
            if self.hibernate(tab.id) {
                count += 1;
            }
        }

        count
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::tabs::{TabStore, TabState};

    fn make_background_tab(store: &Arc<TabStore>, url: &str) -> u32 {
        let id = store.open(url);
        store.set_state(id, TabState::Background);
        id
    }

    #[test]
    fn test_hibernate_background_tab() {
        let store = TabStore::new();
        let id = make_background_tab(&store, "https://google.com");

        let manager = HibernationManager::new(store.clone());
        let result = manager.hibernate(id);

        assert!(result);
        let tabs = store.get_all();
        let tab = tabs.iter().find(|t| t.id == id).unwrap();
        assert_eq!(tab.state, TabState::Hibernated);
    }

    #[test]
    fn test_cannot_hibernate_active_tab() {
        let store = TabStore::new();
        let id = store.open("https://google.com");
        store.set_active(id);

        let manager = HibernationManager::new(store.clone());
        let result = manager.hibernate(id);

        assert!(!result); // should fail
        let tabs = store.get_all();
        let tab = tabs.iter().find(|t| t.id == id).unwrap();
        assert_eq!(tab.state, TabState::Active); // still active
    }

    #[test]
    fn test_wake_hibernated_tab() {
        let store = TabStore::new();
        let id = make_background_tab(&store, "https://google.com");

        let manager = HibernationManager::new(store.clone());
        manager.hibernate(id);

        let url = manager.wake(id);
        assert_eq!(url, Some("https://google.com".to_string()));

        let tabs = store.get_all();
        let tab = tabs.iter().find(|t| t.id == id).unwrap();
        assert_eq!(tab.state, TabState::Loading);
    }

    #[test]
    fn test_gc_pass() {
        let store = TabStore::new();

        // Open 5 background tabs
        for i in 0..5 {
            let id = store.open(&format!("https://site{}.com", i));
            store.set_state(id, TabState::Background);
        }

        // Keep one active
        let active_id = store.open("https://active.com");
        store.set_active(active_id);

        // GC with 0 threshold hibernates all background tabs immediately
        let manager = HibernationManager::new(store.clone())
            .with_threshold(0);

        let hibernated = manager.run_gc_pass();
        assert_eq!(hibernated, 5);

        // Active tab untouched
        let tabs = store.get_all();
        let active = tabs.iter().find(|t| t.id == active_id).unwrap();
        assert_eq!(active.state, TabState::Active);
    }

    #[test]
    fn test_pressure_gc() {
        let store = TabStore::new();
        for i in 0..3 {
            let id = store.open(&format!("https://site{}.com", i));
            store.set_state(id, TabState::Background);
        }

        let manager = HibernationManager::new(store.clone())
            .with_threshold(1000);

        // Normal GC won't hibernate (not idle enough)
        let normal = manager.run_gc_pass();
        assert_eq!(normal, 0);

        // Pressure GC hibernates with lower threshold
        let manager = HibernationManager::new(store.clone())
            .with_threshold(0);
        let pressure = manager.run_pressure_gc();
        assert_eq!(pressure, 3);
    }
}