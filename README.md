# Feather Browser

A lightweight Chromium-based browser built in Rust.
Focused on low RAM usage through aggressive tab hibernation.

## Why Feather?
Chrome uses ~900MB with 10 tabs. Feather targets ~200MB
through tab hibernation and aggressive memory management.

## Status
Active development — Day 8/21 of initial build sprint.

## Stack
- Rust
- wry (WebView2 on Windows / WebKitGTK on Linux)
- winit (native windowing)
- WebView2 (Chromium engine, ships with Windows)

## Building from source
1. Clone the repo
2. Download CEF prebuilt binaries from:
   https://cef-builds.spotifycdn.com/index.html
   Extract into cef/windows/
3. Run:
   cargo build
   cargo run

## Roadmap
- [x] Native window
- [x] Webpage rendering
- [x] Address bar + navigation
- [x] Back/forward history
- [x] Tab data model + state machine
- [ ] Tab hibernation
- [ ] Tab bar UI
- [ ] Memory governor
- [ ] Settings
- [ ] Windows installer (.msi)
- [ ] Linux support