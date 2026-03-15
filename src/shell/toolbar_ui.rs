use egui::{Context, TopBottomPanel, TextEdit, Button, Color32, RichText, Vec2};

pub struct ToolbarState {
    pub address_text: String,
    pub is_loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

impl ToolbarState {
    pub fn new() -> Self {
        Self {
            address_text: "https://duckduckgo.com".to_string(),
            is_loading: false,
            can_go_back: false,
            can_go_forward: false,
        }
    }
}

/// What the user did this frame
pub enum ToolbarEvent {
    Navigate(String),
    GoBack,
    GoForward,
    Reload,
}

/// Draw the toolbar UI, return any event that happened
pub fn draw(ctx: &Context, state: &mut ToolbarState) -> Option<ToolbarEvent> {
    let mut event = None;

    // Dark toolbar background
    let mut style = (*ctx.style()).clone();
    style.visuals.window_fill = Color32::from_rgb(26, 26, 26);
    style.visuals.panel_fill = Color32::from_rgb(26, 26, 26);
    ctx.set_style(style);

    TopBottomPanel::top("toolbar").exact_height(48.0).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            ui.add_space(8.0);

            // Back button
            let back_btn = ui.add_enabled(
                state.can_go_back,
                Button::new(
                    RichText::new("←").color(
                        if state.can_go_back {
                            Color32::WHITE
                        } else {
                            Color32::GRAY
                        }
                    )
                )
                .min_size(Vec2::new(32.0, 32.0))
            );
            if back_btn.clicked() {
                event = Some(ToolbarEvent::GoBack);
            }

            // Forward button
            let fwd_btn = ui.add_enabled(
                state.can_go_forward,
                Button::new(
                    RichText::new("→").color(
                        if state.can_go_forward {
                            Color32::WHITE
                        } else {
                            Color32::GRAY
                        }
                    )
                )
                .min_size(Vec2::new(32.0, 32.0))
            );
            if fwd_btn.clicked() {
                event = Some(ToolbarEvent::GoForward);
            }

            // Reload button
            let reload_label = if state.is_loading { "✕" } else { "↺" };
            let reload_btn = ui.add(
                Button::new(
                    RichText::new(reload_label).color(Color32::WHITE)
                )
                .min_size(Vec2::new(32.0, 32.0))
            );
            if reload_btn.clicked() {
                event = Some(ToolbarEvent::Reload);
            }

            ui.add_space(8.0);

            // Address bar — takes up remaining space
            let available = ui.available_width() - 16.0;
            let addr = ui.add(
                TextEdit::singleline(&mut state.address_text)
                    .desired_width(available)
                    .hint_text("Search or enter address...")
                    .text_color(Color32::WHITE)
                    .frame(true)
            );

            // User hit Enter in address bar
            if addr.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                let url = resolve_input(&state.address_text);
                state.address_text = url.clone();
                event = Some(ToolbarEvent::Navigate(url));
            }

            ui.add_space(8.0);
        });
    });

    event
}

/// Turn user input into a proper URL
fn resolve_input(input: &str) -> String {
    let input = input.trim();

    if input.starts_with("http://") || input.starts_with("https://") {
        return input.to_string();
    }

    if input.contains('.') && !input.contains(' ') {
        return format!("https://{}", input);
    }

    format!("https://duckduckgo.com/?q={}", input.replace(' ', "+"))
}