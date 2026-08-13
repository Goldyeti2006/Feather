const TOOLBAR_SCRIPT: &str = r#"
(function() {
    // Tab state managed in JS, synced from Rust
    window.__feather_tabs = window.__feather_tabs || [
        { id: 1, title: 'New Tab', url: '', active: true, hibernated: false }
    ];
    window.__feather_active_tab = window.__feather_active_tab || 1;

    function inject() {
        if (document.getElementById('feather-root')) return;

        const style = document.createElement('style');
        style.id = 'feather-style';
        style.textContent = `
            html { margin-top: 80px !important; }

            #feather-root {
                position: fixed;
                top: 0; left: 0; right: 0;
                height: 80px;
                z-index: 2147483647;
                font-family: -apple-system, system-ui, sans-serif;
                display: flex;
                flex-direction: column;
            }

            /* ── Tab Bar ── */
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
            .f-tab.active {
                background: #1a1a1a;
                border-color: #333;
            }
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

            /* ── Toolbar ── */
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

        // Root container
        const root = document.createElement('div');
        root.id = 'feather-root';
        root.innerHTML = `
            <div id="feather-tabbar">
                <button id="f-newtab" title="New Tab">+</button>
            </div>
            <div id="feather-toolbar">
                <button id="f-back"   disabled>&#8592;</button>
                <button id="f-fwd"    disabled>&#8594;</button>
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

        // ── Render tabs ──
        window.renderTabs = function() {
            // Remove old tabs
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

                // Switch tab on click
                el.addEventListener('click', (e) => {
                    if (e.target.classList.contains('f-tab-close')) return;
                    window.ipc.postMessage(JSON.stringify({
                        type: 'switch_tab',
                        id: tab.id
                    }));
                });

                // Close tab
                el.querySelector('.f-tab-close').addEventListener('click', () => {
                    window.ipc.postMessage(JSON.stringify({
                        type: 'close_tab',
                        id: tab.id
                    }));
                });

                // Insert before new tab button
                tabbar.insertBefore(el, newTabBtn);
            });
        };

        // New tab
        newTabBtn.addEventListener('click', () => {
            window.ipc.postMessage(JSON.stringify({ type: 'new_tab' }));
        });

        // Address bar
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

        // Nav buttons
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

            // Update active tab title and URL
            const activeTab = window.__feather_tabs
                .find(t => t.active);
            if (activeTab) {
                activeTab.url = location.href;
                activeTab.title = document.title || location.hostname;
                window.renderTabs();
            }
        }

        function resolveInput(input) {
            if (input.startsWith('http://') || input.startsWith('https://')) return input;
            if (input.includes('.') && !input.includes(' ')) return 'https://' + input;
            return 'https://duckduckgo.com/?q=' + encodeURIComponent(input);
        }

        window.addEventListener('load', updateNav);
        window.addEventListener('popstate', updateNav);

        // Called from Rust to update tab list
        window.updateTabs = function(tabs, activeId) {
            window.__feather_tabs = tabs;
            window.__feather_active_tab = activeId;
            window.renderTabs();
        };

        // Initial render
        window.renderTabs();
        addr.value = location.href;
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', inject);
    } else {
        inject();
    }
})();
"#;