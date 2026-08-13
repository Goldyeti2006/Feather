use std::sync::Arc;
use std::thread;
use std::time::Duration;
use crate::shell::tabs::TabStore;
use crate::engine::hibernation::HibernationManager;

pub struct MemoryGovernor {
    store: Arc<TabStore>,
    hibernation: Arc<HibernationManager>,
    /// Check interval in seconds
    check_interval_secs: u64,
    /// RAM threshold before pressure GC fires (in MB)
    ram_threshold_mb: u64,
    /// Seconds idle before normal GC hibernates a tab
    idle_threshold_secs: u64,
}

impl MemoryGovernor {
    pub fn new(
        store: Arc<TabStore>,
        hibernation: Arc<HibernationManager>,
    ) -> Self {
        Self {
            store,
            hibernation,
            check_interval_secs: 10,
            ram_threshold_mb: 800,
            idle_threshold_secs: 300,
        }
    }

    /// Start the governor on a background thread
    /// Returns immediately, governor runs forever in background
    pub fn start(self) {
        thread::spawn(move || {
            log::info!("Memory governor started");
            loop {
                self.tick();
                thread::sleep(Duration::from_secs(self.check_interval_secs));
            }
        });
    }

    fn tick(&self) {
        let total_tabs = self.store.get_all().len();
        let active_count = self.store.active_count();
        let hibernated_count = total_tabs - active_count;
        let total_ram_mb = self.store.total_memory_kb() / 1024;

        log::info!(
            "Memory tick — tabs: {} active / {} hibernated | RAM: ~{}MB",
            active_count, hibernated_count, total_ram_mb
        );

        // Pressure GC — RAM over threshold
        if total_ram_mb > self.ram_threshold_mb {
            log::warn!(
                "RAM {}MB over threshold {}MB — running pressure GC",
                total_ram_mb, self.ram_threshold_mb
            );
            let hibernated = self.hibernation.run_pressure_gc();
            if hibernated > 0 {
                log::info!("Pressure GC hibernated {} tabs", hibernated);
            }
            return;
        }

        // Normal GC — hibernate idle tabs
        let hibernated = self.hibernation.run_gc_pass();
        if hibernated > 0 {
            log::info!("Normal GC hibernated {} tabs", hibernated);
        }
    }
}