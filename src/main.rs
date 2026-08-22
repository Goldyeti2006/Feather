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
    (function() {{
        let lastUrl = location.href;

        function onUrlChange() {{
            window.ipc.postMessage(JSON.stringify({{
                type: 'url_changed',
                tab_id: {tab_id},
                url: location.href,
                title: document.title
            }}));
        }}

        window.addEventListener('load', onUrlChange);

        setInterval(function() {{
            if (location.href !== lastUrl) {{
                lastUrl = location.href;
                onUrlChange();
            }}
        }}, 500);
    }})();
    "#)
}

 const TOOLBAR_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
<style>
    body { margin: 0; font-family: -apple-system, system-ui, sans-serif; background: #1a1a1a; }
    #feather-tabbar {
        height: 32px; background: #111; display: flex; align-items: center;
        padding: 0 4px; gap: 2px; overflow-x: auto; scrollbar-width: none;
    }
    #feather-tabbar::-webkit-scrollbar { display: none; }
    .f-tab {
        height: 26px; max-width: 180px; min-width: 80px; background: #1e1e1e;
        border-radius: 6px 6px 0 0; display: flex; align-items: center; padding: 0 8px;
        gap: 6px; cursor: pointer; flex-shrink: 0; transition: background 0.1s;
        border: 1px solid #2a2a2a; border-bottom: none;
    }
    .f-tab:hover { background: #2a2a2a; }
    .f-tab.active { background: #1a1a1a; border-color: #333; }
    .f-tab-title { flex: 1; font-size: 11px; color: #ccc; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .f-tab.active .f-tab-title { color: #fff; }
    .f-tab-close { font-size: 12px; color: #555; cursor: pointer; padding: 0 2px; border-radius: 3px; flex-shrink: 0; }
    .f-tab-close:hover { color: #fff; background: #444; }
    .f-tab-hibernated { opacity: 0.6; }
    #f-newtab {
        height: 26px; width: 26px; background: transparent; border: none; color: #888;
        font-size: 18px; cursor: pointer; border-radius: 4px; display: flex;
        align-items: center; justify-content: center; flex-shrink: 0; margin-left: 2px;
    }
    #f-newtab:hover { background: #2a2a2a; color: #fff; }
    #feather-toolbar {
        height: 48px; background: #1a1a1a; display: flex; align-items: center;
        padding: 0 8px; gap: 4px; border-bottom: 1px solid #2a2a2a; box-sizing: border-box;
    }
    #feather-toolbar button {
        background: transparent; border: none; color: #fff; font-size: 18px;
        width: 34px; height: 34px; border-radius: 6px; cursor: pointer;
        display: flex; align-items: center; justify-content: center;
        transition: background 0.1s; flex-shrink: 0;
    }
    #feather-toolbar button:hover { background: #2e2e2e; }
    #feather-toolbar button:disabled { color: #444; cursor: default; }
    #feather-toolbar button:disabled:hover { background: transparent; }
    #feather-addr {
        flex: 1; height: 34px; background: #2a2a2a; border: 1.5px solid #333;
        border-radius: 8px; color: #f0f0f0; font-size: 13px; padding: 0 14px;
        outline: none; transition: border-color 0.15s;
    }
    #feather-addr:focus { border-color: #4a9eff; background: #222; }
    #feather-addr::placeholder { color: #555; }
</style>
</head>
<body>
    <div id="feather-tabbar">
        <button id="f-newtab" title="New Tab">+</button>
    </div>
    <div id="feather-toolbar">
        <button id="f-back" disabled>&#8592;</button>
        <button id="f-fwd" disabled>&#8594;</button>
        <button id="f-reload">&#8635;</button>
        <input id="feather-addr" type="text" placeholder="Search or enter address..." spellcheck="false" autocomplete="off" />
    </div>

<script>
    window.__feather_tabs = [];
    window.__feather_active_tab = null;

    const addr      = document.getElementById('feather-addr');
    const backBtn   = document.getElementById('f-back');
    const fwdBtn    = document.getElementById('f-fwd');
    const reloadBtn = document.getElementById('f-reload');
    const tabbar    = document.getElementById('feather-tabbar');
    const newTabBtn = document.getElementById('f-newtab');

    function renderTabs() {
        tabbar.querySelectorAll('.f-tab').forEach(t => t.remove());
        window.__feather_tabs.forEach(tab => {
            const el = document.createElement('div');
            el.className = 'f-tab' + (tab.active ? ' active' : '') + (tab.hibernated ? ' f-tab-hibernated' : '');
            el.dataset.id = tab.id;
            el.innerHTML = `<span class="f-tab-title">${tab.hibernated ? '💤 ' : ''}${tab.title}</span><span class="f-tab-close" data-id="${tab.id}">×</span>`;
            el.addEventListener('click', (e) => {
                if (e.target.classList.contains('f-tab-close')) return;
                window.ipc.postMessage(JSON.stringify({ type: 'switch_tab', id: tab.id }));
            });
            el.querySelector('.f-tab-close').addEventListener('click', () => {
                window.ipc.postMessage(JSON.stringify({ type: 'close_tab', id: tab.id }));
            });
            tabbar.insertBefore(el, newTabBtn);
        });
    }

    newTabBtn.addEventListener('click', () => {
        window.ipc.postMessage(JSON.stringify({ type: 'new_tab' }));
    });

    function resolveInput(input) {
        if (input.startsWith('http://') || input.startsWith('https://')) return input;
        if (input.includes('.') && !input.includes(' ')) return 'https://' + input;
        return 'https://duckduckgo.com/?q=' + encodeURIComponent(input);
    }

    addr.addEventListener('focus', () => addr.select());
    addr.addEventListener('keydown', e => {
        if (e.key === 'Enter') {
            const url = resolveInput(addr.value.trim());
            window.ipc.postMessage(JSON.stringify({ type: 'navigate', url }));
            addr.blur();
        }
    });

    backBtn.addEventListener('click', () => {
    console.log('back button clicked');
    window.ipc.postMessage(JSON.stringify({ type: 'back' }))
    });
    fwdBtn.addEventListener('click', () => {
    console.log('forward button clicked');
    window.ipc.postMessage(JSON.stringify({ type: 'forward' }))});
    reloadBtn.addEventListener('click', () => {
    console.log('reload button clicked');
    window.ipc.postMessage(JSON.stringify({ type: 'reload' }))});

    // Called from Rust
    window.updateTabs = function(tabs, activeId) {
        window.__feather_tabs = tabs;
        window.__feather_active_tab = activeId;
        renderTabs();
    };

    window.updateAddress = function(url) {
        addr.value = url;
    };

    window.updateNavButtons = function(canBack, canForward) {
        backBtn.disabled = !canBack;
        fwdBtn.disabled = !canForward;
    };
</script>
</body>
</html>
"#;
fn sync_tabs_to_js(webview: &wry::WebView, tab_store: &TabStore) {
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

    let (can_back, can_forward) = tab_store.nav_state(active_id);
    let nav_script = format!("window.updateNavButtons({}, {})", can_back, can_forward);
    let _ = webview.evaluate_script(&nav_script);
}
fn resize_webviews(toolbar: &wry::WebView,
    content: &std::collections::HashMap<u32, wry::WebView>,
    width: u32,
    height: u32,){
    let _ = toolbar.set_bounds(wry::Rect {
        x: 0, y: 0, width, height: 80,
    });

    for wv in content.values() {
        let _ = wv.set_bounds(wry::Rect {
            x: 0, y: 80, width, height: height.saturating_sub(80),
        });
    }
}

fn load_icon(path: &str) -> winit::window::Icon {
    let img = image::open(path).expect("failed to load icon").into_rgba8();
    let (width, height) = img.dimensions();
    winit::window::Icon::from_rgba(img.into_raw(), width, height)
        .expect("failed to create icon")
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
    let icon = load_icon("assets/feather.ico");
    let window = WindowBuilder::new()
        .with_title("Feather")
        .with_window_icon(Some(icon))
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();
    
    
    // NEW: track content-area size (window size minus toolbar height)
    let inner = window.inner_size(); // PhysicalSize<u32>
    let mut window_size: (u32, u32) = (inner.width, inner.height);

    // IPC channel — toolbar sends messages here
    let (toolbar_tx, toolbar_rx) = mpsc::channel::<String>();
    let (content_tx, content_rx) = mpsc::channel::<String>();
    
    // toolbar experiment
    let toolbar_webview = WebViewBuilder::new_as_child(&window)
    .with_devtools(true)
    .with_html(TOOLBAR_HTML)
    .with_bounds(wry::Rect { x: 0, y: 0, width: 1280, height: 80 })
    .with_ipc_handler(move |msg| {
        let _ = toolbar_tx.send(msg);
    })
    .build()
    .unwrap();

    // Content WebViews — one per tab
    let mut content_webviews: std::collections::HashMap<u32, wry::WebView> = std::collections::HashMap::new();

    // Create the first tab's WebView (matches first_tab opened earlier)
    {
        let tx = content_tx.clone();
        let id = first_tab;
        let wv = WebViewBuilder::new_as_child(&window)
            .with_url("https://duckduckgo.com")
            .with_bounds(wry::Rect { x: 0, y: 80, width: 1200, height: 720 })
            .with_initialization_script(&url_sync_script(id))
            .with_ipc_handler(move |msg| {
                let _ = tx.send(msg);
            })
            .build()
            .unwrap();
        content_webviews.insert(id, wv);
    }
    sync_tabs_to_js(&toolbar_webview, &tab_store);

    log::info!("Feather ready");

    // Run event loop
    event_loop.run(move |event, elwt| {
    elwt.set_control_flow(ControlFlow::Poll);

    // --- Toolbar IPC: navigate, back, forward, reload, new_tab, close_tab, switch_tab ---
        if let Ok(msg) = toolbar_rx.try_recv() {
            log::info!("RAW toolbar message: {}", msg);
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                match val["type"].as_str() {
                    Some("navigate") => {
                        let url = val["url"].as_str().unwrap_or("").to_string();
                        let active = tab_store.active_id();
                        if let Some(wv) = content_webviews.get(&active) {
                            let _ = wv.load_url(&url);
                        }
                        sync_tabs_to_js(&toolbar_webview, &tab_store);
                    }
                    Some("new_tab") => {
                        let id = tab_store.open("https://duckduckgo.com");
                        tab_store.set_active(id);

                        // hide all other content webviews
                        for (tab_id, wv) in content_webviews.iter() {
                            let _ = wv.set_visible(*tab_id == id);
                        }

                        let tx = content_tx.clone();
                        let wv = WebViewBuilder::new_as_child(&window)
                            .with_url("https://duckduckgo.com")
                            .with_bounds(wry::Rect { x: 0, y: 80, width: window_size.0, height: window_size.1.saturating_sub(80) })
                            .with_initialization_script(&url_sync_script(id))
                            .with_ipc_handler(move |msg| {
                                let _ = tx.send(msg);
                            })
                            .build()
                            .unwrap();
                        content_webviews.insert(id, wv);

                        sync_tabs_to_js(&toolbar_webview, &tab_store);
                    }
                    Some("close_tab") => {
                        if let Some(id) = val["id"].as_u64() {
                            let id = id as u32;
                            tab_store.close(id);
                            content_webviews.remove(&id);
                            sync_tabs_to_js(&toolbar_webview, &tab_store);
                        }
                    }
                    Some("switch_tab") => {
                        if let Some(id) = val["id"].as_u64() {
                            let id = id as u32;
                            tab_store.set_active(id);
                            for (tab_id, wv) in content_webviews.iter() {
                                let _ = wv.set_visible(*tab_id == id);
                            }
                            sync_tabs_to_js(&toolbar_webview, &tab_store);
                        }
                    }
                    Some("back") => {
                        let active = tab_store.active_id();
                        if let Some(new_url) = tab_store.go_back(active) {
                            tab_store.set_suppress(active, true);
                            if let Some(wv) = content_webviews.get(&active) {
                                let _ = wv.load_url(&new_url);
                            }
                            sync_tabs_to_js(&toolbar_webview, &tab_store);
                        }
                    }
                    Some("forward") => {
                        let active = tab_store.active_id();
                        if let Some(new_url) = tab_store.go_forward(active) {
                            tab_store.set_suppress(active, true);
                            if let Some(wv) = content_webviews.get(&active) {
                                let _ = wv.load_url(&new_url);
                            }
                            sync_tabs_to_js(&toolbar_webview, &tab_store);
                        }
                    }
                    Some("reload") => {
                        let active = tab_store.active_id();
                        tab_store.set_suppress(active, true);
                        if let Some(wv) = content_webviews.get(&active) {
                            let _ = wv.evaluate_script("location.reload()");
                        }
                    }
                    _ => {}
                }
            }
        }

        // --- Content IPC: url_changed (from whichever tab's WebView sent it) ---
        if let Ok(msg) = content_rx.try_recv() {
             log::info!("RAW content message: {}", msg);
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                if val["type"].as_str() == Some("url_changed") {
                    if let Some(tab_id) = val["tab_id"].as_u64() {
                        let tab_id = tab_id as u32;
                        let url = val["url"].as_str().unwrap_or("").to_string();
                        let title = val["title"].as_str().unwrap_or("New Tab").to_string();

                        if tab_store.take_suppress(tab_id) {
                            // this load was caused by back/forward/reload — don't push new history
                            tab_store.set_title(tab_id, &title);
                        } else {
                            tab_store.set_url(tab_id, &url);
                            tab_store.set_title(tab_id, &title);
                        }

                        if tab_id == tab_store.active_id() {
                            let script = format!("window.updateAddress('{}')", url);
                            let _ = toolbar_webview.evaluate_script(&script);
                            window.set_title(&title);
                        }
                        sync_tabs_to_js(&toolbar_webview, &tab_store);
                    }
                }
            }
        }

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                elwt.exit();
            }
            Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
                window_size = (physical_size.width, physical_size.height);
                resize_webviews(&toolbar_webview, &content_webviews, window_size.0, window_size.1);
            }
            _ => {}
        }

        let _ = &hibernation;
    }).unwrap();
}