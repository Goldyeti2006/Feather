mod engine;
mod shell;
mod platform;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use shell::tabs::TabStore;
use wry::WebViewBuilder;
use std::sync::mpsc;
// This script is injected into every page that loads
// It creates a fixed toolbar at the top and pushes page content down
const TOOLBAR_SCRIPT: &str = r#"
(function() {
    function inject() {
        if (document.getElementById('feather-toolbar')) return;

        const style = document.createElement('style');
        style.id = 'feather-style';
        style.textContent = `
            html { margin-top: 48px !important; }
            #feather-toolbar {
                position: fixed;
                top: 0; left: 0; right: 0;
                height: 48px;
                background: #1a1a1a;
                display: flex;
                align-items: center;
                padding: 0 8px;
                gap: 4px;
                z-index: 2147483647;
                font-family: -apple-system, system-ui, sans-serif;
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

        const toolbar = document.createElement('div');
        toolbar.id = 'feather-toolbar';
        toolbar.innerHTML = `
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
        `;
        document.documentElement.appendChild(toolbar);

        const addr      = document.getElementById('feather-addr');
        const backBtn   = document.getElementById('f-back');
        const fwdBtn    = document.getElementById('f-fwd');
        const reloadBtn = document.getElementById('f-reload');

        addr.value = location.href;

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

        function resolveInput(input) {
            if (input.startsWith('http://') || input.startsWith('https://')) return input;
            if (input.includes('.') && !input.includes(' ')) return 'https://' + input;
            return 'https://duckduckgo.com/?q=' + encodeURIComponent(input);
        }

        function updateNav() {
            backBtn.disabled = (history.length <= 1);
            fwdBtn.disabled = false;
            addr.value = location.href;
        }

        window.addEventListener('load', updateNav);
        window.addEventListener('popstate', updateNav);
        updateNav();
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', inject);
    } else {
        inject();
    }
})();
"#;

fn main() {
    env_logger::init();
    log::info!("Feather starting...");

    let flags = engine::cef_app::performance_flags();
    log::info!("Loaded {} performance flags", flags.len());

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("Feather")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();
let (tx, rx) = mpsc::channel::<String>();
let tab_store = TabStore::new();
let first_tab = tab_store.open("https://duckduckgo.com");
tab_store.set_active(first_tab);

log::info!(
    "Tab store initialized — {} tab open",
    tab_store.get_all().len()
);
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

    event_loop.run(move |event, elwt| {
    elwt.set_control_flow(ControlFlow::Poll);

    // Check for toolbar commands
    if let Ok(msg) = rx.try_recv() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
            match val["type"].as_str() {
                Some("navigate") => {
                    let url = val["url"].as_str().unwrap_or("").to_string();
                    log::info!("Navigate: {}", url);
                    let _ = webview.load_url(&url);
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

    match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested, ..
        } => {
            log::info!("Shutting down Feather");
            elwt.exit();
        }
        _ => {}
    }

    let _ = &webview;
}).unwrap();
}