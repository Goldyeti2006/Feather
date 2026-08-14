mod engine;
mod shell;
mod platform;
use engine::memory::MemoryGovernor;
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;
use std::sync::mpsc;
use engine::hibernation::HibernationManager;
use shell::tabs::{TabStore, TabState};

fn url_sync_script(tab_id: u32) -> String {
    format!(r#"
    console.log('script injected for tab', {tab_id});
    window.addEventListener('load',function() {{
        console.log('sending url_changed for tab', {tab_id});
        window.ipc.postMessage(JSON.stringify({{
         type: 'url_changed',
         tab_id: {tab_id},
         url: location.href,
         title: document.title}}));
         }});
          "#)
}

 const TOOLBAR_SCRIPT: &str = r#"
(function() {
    window.__feather_tabs = window.__feather_tabs || [
        { id: 1, title: 'New Tab', url: '', active: true, hibernated: false }
    ];
    window.__feather_active_tab = window.__feather_active_tab || 1;

    function inject() {
        if (document.getElementById('feather-root')) return;

        const style = document.createElement('style');
        style.id = 'feather-style';
        style.textContent = `
            html {
                margin-top: 80px !important;
                visibility: hidden;
                animation: feather-fadein 0.08s ease forwards;
            }
            @keyframes feather-fadein {
                from { opacity: 0; }
                to   { opacity: 1; }
            }
            #feather-root {
                visibility: visible !important;
                opacity: 1 !important;
                position: fixed;
                top: 0; left: 0; right: 0;
                height: 80px;
                z-index: 2147483647;
                font-family: -apple-system, system-ui, sans-serif;
                display: flex;
                flex-direction: column;
            }
            #feather-tabbar {
                height: 32px;
                background: #111;
                display: flex;
                align-items: center;
                padding: 0 4px;
                gap: 2px;
                overflow-x: auto;
                scrollbar-width: none;
            }
            #feather-tabbar::-webkit-scrollbar { display: none; }
            .f-tab {
                height: 26px;
                max-width: 180px;
                min-width: 80px;
                background: #1e1e1e;
                border-radius: 6px 6px 0 0;
                display: flex;
                align-items: center;
                padding: 0 8px;
                gap: 6px;
                cursor: pointer;
                flex-shrink: 0;
                transition: background 0.1s;
                border: 1px solid #2a2a2a;
                border-bottom: none;
            }
            .f-tab:hover { background: #2a2a2a; }
            .f-tab.active { background: #1a1a1a; border-color: #333; }
            .f-tab-title {
                flex: 1;
                font-size: 11px;
                color: #ccc;
                white-space: nowrap;
                overflow: hidden;
                text-overflow: ellipsis;
            }
            .f-tab.active .f-tab-title { color: #fff; }
            .f-tab-close {
                font-size: 12px;
                color: #555;
                cursor: pointer;
                padding: 0 2px;
                border-radius: 3px;
                flex-shrink: 0;
            }
            .f-tab-close:hover { color: #fff; background: #444; }
            .f-tab-hibernated { opacity: 0.6; }
            #f-newtab {
                height: 26px;
                width: 26px;
                background: transparent;
                border: none;
                color: #888;
                font-size: 18px;
                cursor: pointer;
                border-radius: 4px;
                display: flex;
                align-items: center;
                justify-content: center;
                flex-shrink: 0;
                margin-left: 2px;
            }
            #f-newtab:hover { background: #2a2a2a; color: #fff; }
            #feather-toolbar {
                height: 48px;
                background: #1a1a1a;
                display: flex;
                align-items: center;
                padding: 0 8px;
                gap: 4px;
                border-bottom: 1px solid #2a2a2a;
                box-sizing: border-box;
            }
            #feather-toolbar button {
                background: transparent;
                border: none;
                color: #fff;
                font-size: 18px;
                width: 34px;
                height: 34px;
                border-radius: 6px;
                cursor: pointer;
                display: flex;
                align-items: center;
                justify-content: center;
                transition: background 0.1s;
                flex-shrink: 0;
            }
            #feather-toolbar button:hover { background: #2e2e2e; }
            #feather-toolbar button:disabled { color: #444; cursor: default; }
            #feather-toolbar button:disabled:hover { background: transparent; }
            #feather-addr {
                flex: 1;
                height: 34px;
                background: #2a2a2a;
                border: 1.5px solid #333;
                border-radius: 8px;
                color: #f0f0f0;
                font-size: 13px;
                padding: 0 14px;
                outline: none;
                transition: border-color 0.15s;
            }
            #feather-addr:focus { border-color: #4a9eff; background: #222; }
            #feather-addr::placeholder { color: #555; }
        `;
        document.documentElement.appendChild(style);

        const root = document.createElement('div');
        root.id = 'feather-root';
        root.innerHTML = `
            <div id="feather-tabbar">
                <button id="f-newtab" title="New Tab">+</button>
            </div>
            <div id="feather-toolbar">
                <button id="f-back" disabled>&#8592;</button>
                <button id="f-fwd"  disabled>&#8594;</button>
                <button id="f-reload">&#8635;</button>
                <input
                    id="feather-addr"
                    type="text"
                    placeholder="Search or enter address..."
                    spellcheck="false"
                    autocomplete="off"
                />
            </div>
        `;
        document.documentElement.appendChild(root);

        const addr      = document.getElementById('feather-addr');
        const backBtn   = document.getElementById('f-back');
        const fwdBtn    = document.getElementById('f-fwd');
        const reloadBtn = document.getElementById('f-reload');
        const tabbar    = document.getElementById('feather-tabbar');
        const newTabBtn = document.getElementById('f-newtab');

        window.renderTabs = function() {
            tabbar.querySelectorAll('.f-tab').forEach(t => t.remove());
            window.__feather_tabs.forEach(tab => {
                const el = document.createElement('div');
                el.className = 'f-tab' +
                    (tab.active ? ' active' : '') +
                    (tab.hibernated ? ' f-tab-hibernated' : '');
                el.dataset.id = tab.id;
                el.innerHTML = `
                    <span class="f-tab-title">
                        ${tab.hibernated ? '💤 ' : ''}${tab.title}
                    </span>
                    <span class="f-tab-close" data-id="${tab.id}">×</span>
                `;
                el.addEventListener('click', (e) => {
                    if (e.target.classList.contains('f-tab-close')) return;
                    window.ipc.postMessage(JSON.stringify({
                        type: 'switch_tab', id: tab.id
                    }));
                });
                el.querySelector('.f-tab-close').addEventListener('click', () => {
                    window.ipc.postMessage(JSON.stringify({
                        type: 'close_tab', id: tab.id
                    }));
                });
                tabbar.insertBefore(el, newTabBtn);
            });
        };

        newTabBtn.addEventListener('click', () => {
            window.ipc.postMessage(JSON.stringify({ type: 'new_tab' }));
        });

        addr.addEventListener('focus', () => addr.select());
        addr.addEventListener('keydown', e => {
            if (e.key === 'Enter') {
                const url = resolveInput(addr.value.trim());
                addr.value = url;
                window.ipc.postMessage(JSON.stringify({ type: 'navigate', url }));
                addr.blur();
            }
            if (e.key === 'Escape') {
                addr.value = location.href;
                addr.blur();
            }
        });

        backBtn.addEventListener('click', () => {
            window.ipc.postMessage(JSON.stringify({ type: 'back' }));
        });
        fwdBtn.addEventListener('click', () => {
            window.ipc.postMessage(JSON.stringify({ type: 'forward' }));
        });
        reloadBtn.addEventListener('click', () => {
            window.ipc.postMessage(JSON.stringify({ type: 'reload' }));
        });

        function updateNav() {
            backBtn.disabled = (history.length <= 1);
            fwdBtn.disabled = false;
            addr.value = location.href;
            const activeTab = window.__feather_tabs.find(t => t.active);
            if (activeTab) {
                activeTab.url = location.href;
                activeTab.title = document.title || location.hostname;
                window.renderTabs();
            }
            // Tell Rust the current URL so tab store stays in sync
            window.ipc.postMessage(JSON.stringify({
                type: 'url_changed',
                url: location.href,
                title: document.title || location.hostname
            }));
        }

        function resolveInput(input) {
            if (input.startsWith('http://') || input.startsWith('https://')) return input;
            if (input.includes('.') && !input.includes(' ')) return 'https://' + input;
            return 'https://duckduckgo.com/?q=' + encodeURIComponent(input);
        }

        window.addEventListener('load', updateNav);
        window.addEventListener('popstate', updateNav);

        window.updateTabs = function(tabs, activeId) {
            window.__feather_tabs = tabs;
            window.__feather_active_tab = activeId;
            window.renderTabs();
        };

        window.renderTabs();
        addr.value = location.href;
        document.documentElement.style.visibility = 'visible';
        document.documentElement.style.opacity = '1';
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', inject);
    } else {
        inject();
        document.addEventListener('DOMContentLoaded', inject);
    }
})();
"#;

fn sync_tabs_to_js(
    webview: &wry::WebView,
    tab_store: &TabStore,
) {
   let tabs = tab_store.get_all();
    let active_id = tab_store.active_id();
    let tabs_json = serde_json::json!(
        tabs.iter().map(|t| serde_json::json!({
            "id": t.id,
            "title": t.title,
            "url": t.url,
            "active": t.id == active_id,
            "hibernated": t.state == TabState::Hibernated,
        })).collect::<Vec<_>>()
    );
    let script = format!("window.updateTabs({}, {})", tabs_json, active_id);
    let _ = webview.evaluate_script(&script);
}

fn main() {
    env_logger::init();
    log::info!("Feather starting...");

    // Load performance flags
    let flags = engine::cef_app::performance_flags();
    log::info!("Loaded {} performance flags", flags.len());

    // Set up tab store
    let tab_store = TabStore::new();
    let first_tab = tab_store.open("https://duckduckgo.com");
    tab_store.set_active(first_tab);
    log::info!("Tab store ready — {} tab open", tab_store.get_all().len());

    // Set up hibernation manager
    let hibernation = Arc::new(
        HibernationManager::new(tab_store.clone())
            .with_threshold(300)
    );
    log::info!("Hibernation manager ready (threshold: 300s)");

    // Start memory governor on background thread
    let governor = MemoryGovernor::new(
        tab_store.clone(),
        hibernation.clone(),
    );
    governor.start();
    log::info!("Memory governor started (checks every 10s)");

    // Create window
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Feather")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    // IPC channel — toolbar sends messages here
    let (tx, rx) = mpsc::channel::<String>();

    // Create WebView
    let webview = WebViewBuilder::new(&window)
        .with_url("https://duckduckgo.com")
        .with_initialization_script(TOOLBAR_SCRIPT)
        .with_devtools(true)
        .with_ipc_handler(move |msg| {
            let _ = tx.send(msg);
        })
        .build()
        .unwrap();

    log::info!("Feather ready");

    // Run event loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        // Handle IPC messages from toolbar
        if let Ok(msg) = rx.try_recv() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                match val["type"].as_str() {
                    Some("navigate") => {
                        let url = val["url"].as_str().unwrap_or("").to_string();
                        log::info!("Navigate: {}", url);
                        let _ = webview.load_url(&url);
                        // Update active tab URL in store
                        let active = tab_store.active_id();
                        tab_store.set_url(active, &url);
                        sync_tabs_to_js(&webview, &tab_store);
                    }
                    Some("new_tab") => {
                        let id = tab_store.open("https://duckduckgo.com");
                        tab_store.set_active(id);
                        let _ = webview.load_url("https://duckduckgo.com");
                        log::info!("New tab: {}", id);
                        sync_tabs_to_js(&webview, &tab_store);
                    }
                    Some("close_tab") => {
                        if let Some(id) = val["id"].as_u64() {
                            tab_store.close(id as u32);
                            log::info!("Closed tab: {}", id);
                            sync_tabs_to_js(&webview, &tab_store);
                        }
                    }
                    Some("switch_tab") => {
                        if let Some(id) = val["id"].as_u64() {
                            let id = id as u32;
                            let tabs = tab_store.get_all();
                            if let Some(tab) = tabs.iter().find(|t| t.id == id) {
                                if tab.state == TabState::Hibernated {
                                    if let Some(url) = hibernation.wake(id) {
                                        tab_store.set_active(id);
                                        let _ = webview.load_url(&url);
                                        log::info!("Woke tab {}", id);
                                    }
                                } else {
                                    tab_store.set_active(id);
                                    let _ = webview.load_url(&tab.url);
                                }
                            }
                            sync_tabs_to_js(&webview, &tab_store);
                        }
                    }
                    Some("url_changed") => {
                        let url = val["url"].as_str().unwrap_or("").to_string();
                        let title = val["title"].as_str().unwrap_or("New Tab").to_string();
                        let active = tab_store.active_id();
                        tab_store.set_url(active, &url);
                        tab_store.set_title(active, &title);
                        sync_tabs_to_js(&webview, &tab_store)
                    }
                    Some("back") => {
                        let _ = webview.evaluate_script("window.history.go(-1)");
                    }
                    Some("forward") => {
                        let _ = webview.evaluate_script("window.history.go(1)");
                    }
                    Some("reload") => {
                        let _ = webview.evaluate_script("location.reload()");
                    }
                    _ => {}
                }
            }
        }

        // Handle window events
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested, ..
            } => {
                log::info!("Shutting down Feather");
                elwt.exit();
            }
            _ => {}
        }

        // Keep everything alive
        let _ = &webview;
        let _ = &tab_store;
        let _ = &hibernation;
    }).unwrap();
}