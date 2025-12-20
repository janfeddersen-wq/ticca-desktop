//! Animated spinner widget using iced canvas

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Element, Length, Point, Renderer, Theme};
use std::f32::consts::PI;

/// A smooth animated spinner widget
pub struct Spinner {
    frame: usize,
}

impl Spinner {
    pub fn new(frame: usize) -> Self {
        Self { frame }
    }
}

impl<Message> canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (bounds.width.min(bounds.height) / 2.0) - 2.0;

        // Get theme-appropriate color
        let palette = theme.extended_palette();
        let color = palette.primary.base.color;

        // Background circle (faint)
        let bg_circle = Path::circle(center, radius);
        frame.stroke(
            &bg_circle,
            Stroke::default()
                .with_color(iced::Color { a: 0.2, ..color })
                .with_width(2.0),
        );

        // Animated arc - rotates based on frame
        // Each frame advances by ~6 degrees (60 frames = full rotation)
        let rotation = (self.frame as f32 * 6.0) * PI / 180.0;

        // Draw an arc that spans about 90 degrees
        let arc_length = PI / 2.0; // 90 degrees
        let start_angle = rotation;

        // Create arc path
        let arc = Path::new(|builder| {
            let start = Point::new(
                center.x + radius * start_angle.cos(),
                center.y + radius * start_angle.sin(),
            );
            builder.move_to(start);

            // Approximate arc with bezier curves
            let segments = 8;
            let angle_step = arc_length / segments as f32;

            for i in 0..segments {
                let a1 = start_angle + (i as f32) * angle_step;
                let a2 = start_angle + ((i + 1) as f32) * angle_step;

                // Control points for cubic bezier approximation of arc
                let k = 4.0 / 3.0 * ((a2 - a1) / 4.0).tan();

                let p1 = Point::new(center.x + radius * a1.cos(), center.y + radius * a1.sin());
                let p2 = Point::new(center.x + radius * a2.cos(), center.y + radius * a2.sin());

                let c1 = Point::new(p1.x - k * radius * a1.sin(), p1.y + k * radius * a1.cos());
                let c2 = Point::new(p2.x + k * radius * a2.sin(), p2.y - k * radius * a2.cos());

                builder.bezier_curve_to(c1, c2, p2);
            }
        });

        frame.stroke(
            &arc,
            Stroke::default()
                .with_color(color)
                .with_width(2.0)
                .with_line_cap(canvas::LineCap::Round),
        );

        vec![frame.into_geometry()]
    }
}

/// Create a spinner element
pub fn spinner<'a, Message: 'a>(frame: usize) -> Element<'a, Message> {
    Canvas::new(Spinner::new(frame))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .into()
}
