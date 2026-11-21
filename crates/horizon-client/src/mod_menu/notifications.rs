//! Corner toast notifications, CorzClient style: an accent bar, a title and a
//! short message that slide in, wait and fade out.

use super::theme::{self, Palette};
use horizon_client_render::hud::{HudAtlas, HudBatch, rgba};

const LIFE: f32 = 3.4;
const MAX: usize = 4;

#[derive(Clone, Debug)]
pub struct Toast {
    pub title: String,
    pub message: String,
    pub accent: u32,
    pub age: f32,
    /// Whether this toast already played its slide-in sound.
    pub born: bool,
}

#[derive(Default)]
pub struct Notifications {
    pub toasts: Vec<Toast>,
}

impl Notifications {
    pub fn push(&mut self, title: &str, message: &str, accent: u32) {
        if self.toasts.len() >= MAX {
            self.toasts.remove(0);
        }
        self.toasts.push(Toast {
            title: title.to_string(),
            message: message.to_string(),
            accent,
            age: 0.0,
            born: false,
        });
    }

    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    pub fn tick(&mut self, dt: f32) {
        for t in &mut self.toasts {
            t.age += dt.clamp(0.0, 0.25);
        }
        self.toasts.retain(|t| t.age < LIFE);
    }

    /// Draws every toast at `anchor` (0 top-right, 1 top-left, 2 bottom-right,
    /// 3 bottom-left). Returns the height used.
    pub fn draw(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        palette: &Palette,
        anchor: usize,
        screen_w: f32,
        screen_h: f32,
        animations: bool,
    ) {
        if self.toasts.is_empty() {
            return;
        }
        let w = 168.0;
        let h = 38.0;
        let gap = 6.0;
        let margin = 8.0;
        let right = anchor == 0 || anchor == 2;
        let bottom = anchor >= 2;
        let mut y = if bottom {
            screen_h - margin - h
        } else {
            margin
        };
        for t in self.toasts.iter().rev() {
            let fade_in = if animations {
                (t.age / 0.18).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let fade_out = ((LIFE - t.age) / 0.35).clamp(0.0, 1.0);
            let alpha = fade_in.min(fade_out);
            let slide = (1.0 - fade_in) * 22.0;
            let x = if right {
                screen_w - margin - w + slide
            } else {
                margin - slide
            };
            let a = |c: u32| theme::scale_alpha(c, alpha);

            theme::shadow(batch, atlas, x, y, w, h, 8.0, rgba(0, 0, 0, 150));
            theme::rounded_rect(batch, atlas, x, y, w, h, 8.0, a(palette.header));
            batch.fill(atlas, x, y + 4.0, 2.5, h - 8.0, a(t.accent));
            batch.text(
                atlas,
                &t.title,
                x + 10.0,
                y + 7.0,
                a(palette.text),
                true,
            );
            let msg = truncate(&t.message, 30);
            batch.text(atlas, &msg, x + 10.0, y + 21.0, a(palette.text_dim), false);
            y += if bottom { -(h + gap) } else { h + gap };
        }
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
