//! Agent call graph view (React Flow-style panel).

use iced::widget::{canvas, column, container, row, scrollable, text};
use iced::{Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size, alignment};

use ticca_core::agents::AgentRegistry;

use crate::agent_graph::{AgentCallGraph, AgentNode, AgentStatus};
use crate::material_icons::{icon, icons};
use crate::messages::Message;
use crate::theme::{AppTheme, styles};

pub fn contents<'a>(
    graph: &AgentCallGraph,
    theme: AppTheme,
    animation_frame: usize,
) -> Element<'a, Message> {
    let current_run = graph.current_run_id();
    let run_text = if current_run > 0 {
        format!("Live call graph • Run #{}", current_run)
    } else {
        "Live call graph".to_string()
    };

    let subtitle = row![icon(icons::FORUM).size(16), text(run_text).size(11),]
        .spacing(6)
        .align_y(iced::Alignment::Center);

    let canvas = canvas::Canvas::new(FlowCanvas::new(graph.clone(), theme, animation_frame))
        .width(Length::Fill)
        .height(Length::Fill);

    column![subtitle, scrollable(canvas).height(Length::Fill)]
        .spacing(6)
        .padding(12)
        .into()
}

#[allow(dead_code)]
pub fn view<'a>(
    graph: &AgentCallGraph,
    theme: AppTheme,
    animation_frame: usize,
) -> Element<'a, Message> {
    container(contents(graph, theme, animation_frame))
        .width(Length::Fixed(280.0))
        .height(Length::Fill)
        .style(styles::flow_panel_container)
        .into()
}

#[derive(Debug, Clone)]
struct FlowCanvas {
    graph: AgentCallGraph,
    theme: AppTheme,
    animation_frame: usize,
}

impl FlowCanvas {
    fn new(graph: AgentCallGraph, theme: AppTheme, animation_frame: usize) -> Self {
        Self {
            graph,
            theme,
            animation_frame,
        }
    }

    fn is_dark(&self) -> bool {
        self.theme.is_dark()
    }

    fn node_color(&self, node: &AgentNode) -> iced::Color {
        let (r, g, b) = AgentRegistry::get(node.agent_type).color;
        let base = iced::Color::from_rgb(r, g, b);

        match node.status {
            AgentStatus::Running => base, // Full opacity
            AgentStatus::Completed => iced::Color { a: 0.85, ..base },
            AgentStatus::Failed => iced::Color::from_rgb(0.9, 0.3, 0.3), // Red tint
        }
    }

    fn text_color(&self) -> iced::Color {
        if self.is_dark() {
            iced::Color::from_rgb(0.95, 0.95, 0.95)
        } else {
            iced::Color::from_rgb(0.10, 0.10, 0.10)
        }
    }

    fn secondary_text_color(&self) -> iced::Color {
        if self.is_dark() {
            iced::Color::from_rgb(0.70, 0.70, 0.70)
        } else {
            iced::Color::from_rgb(0.40, 0.40, 0.40)
        }
    }

    fn edge_color(&self) -> iced::Color {
        if self.is_dark() {
            iced::Color::from_rgb(0.35, 0.45, 0.55)
        } else {
            iced::Color::from_rgb(0.55, 0.60, 0.70)
        }
    }

    fn grid_color(&self) -> iced::Color {
        if self.is_dark() {
            iced::Color::from_rgba(0.30, 0.30, 0.35, 0.25)
        } else {
            iced::Color::from_rgba(0.60, 0.60, 0.65, 0.25)
        }
    }

    /// Draw a status indicator in the top-right corner of a node
    fn draw_status_indicator(
        &self,
        frame: &mut canvas::Frame,
        position: Point,
        node_width: f32,
        status: AgentStatus,
    ) {
        let indicator_center = Point::new(position.x + node_width - 12.0, position.y + 12.0);

        match status {
            AgentStatus::Running => {
                // Draw spinning arc
                let rotation = (self.animation_frame as f32 * 0.15) % (2.0 * std::f32::consts::PI);
                let arc_radius = 5.0;

                // Draw arc background (subtle circle)
                let bg_circle = canvas::Path::circle(indicator_center, arc_radius);
                frame.stroke(
                    &bg_circle,
                    canvas::Stroke::default()
                        .with_width(2.0)
                        .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.2)),
                );

                // Draw spinning arc segment
                let arc = canvas::Path::new(|builder| {
                    let segments = 8;
                    let arc_length = std::f32::consts::PI * 0.75;
                    for i in 0..=segments {
                        let angle = rotation + (i as f32 / segments as f32) * arc_length;
                        let x = indicator_center.x + angle.cos() * arc_radius;
                        let y = indicator_center.y + angle.sin() * arc_radius;
                        if i == 0 {
                            builder.move_to(Point::new(x, y));
                        } else {
                            builder.line_to(Point::new(x, y));
                        }
                    }
                });
                frame.stroke(
                    &arc,
                    canvas::Stroke::default()
                        .with_width(2.0)
                        .with_color(Color::from_rgb(1.0, 1.0, 1.0)),
                );
            }
            AgentStatus::Completed => {
                // Draw checkmark
                let check = canvas::Path::new(|builder| {
                    builder.move_to(Point::new(indicator_center.x - 4.0, indicator_center.y));
                    builder.line_to(Point::new(
                        indicator_center.x - 1.0,
                        indicator_center.y + 3.0,
                    ));
                    builder.line_to(Point::new(
                        indicator_center.x + 4.0,
                        indicator_center.y - 3.0,
                    ));
                });
                frame.stroke(
                    &check,
                    canvas::Stroke::default()
                        .with_width(2.0)
                        .with_color(Color::from_rgb(0.2, 0.8, 0.3)),
                );
            }
            AgentStatus::Failed => {
                // Draw X
                let x_path = canvas::Path::new(|builder| {
                    builder.move_to(Point::new(
                        indicator_center.x - 4.0,
                        indicator_center.y - 4.0,
                    ));
                    builder.line_to(Point::new(
                        indicator_center.x + 4.0,
                        indicator_center.y + 4.0,
                    ));
                    builder.move_to(Point::new(
                        indicator_center.x + 4.0,
                        indicator_center.y - 4.0,
                    ));
                    builder.line_to(Point::new(
                        indicator_center.x - 4.0,
                        indicator_center.y + 4.0,
                    ));
                });
                frame.stroke(
                    &x_path,
                    canvas::Stroke::default()
                        .with_width(2.0)
                        .with_color(Color::from_rgb(1.0, 0.4, 0.4)),
                );
            }
        }
    }

    /// Draw duration/elapsed text below the node label
    fn draw_duration_text(
        &self,
        frame: &mut canvas::Frame,
        node: &AgentNode,
        position: Point,
        node_width: f32,
        node_height: f32,
    ) {
        let duration_text = match node.status {
            AgentStatus::Running => node.elapsed().map(|d| {
                let secs = d.as_secs();
                if secs < 60 {
                    format!("{}s", secs)
                } else {
                    format!("{}m {}s", secs / 60, secs % 60)
                }
            }),
            AgentStatus::Completed | AgentStatus::Failed => node.duration().map(|d| {
                let secs = d.as_secs();
                if secs < 60 {
                    format!("took {}s", secs)
                } else {
                    format!("took {}m {}s", secs / 60, secs % 60)
                }
            }),
        };

        if let Some(text_content) = duration_text {
            frame.fill_text(canvas::Text {
                content: text_content,
                position: Point::new(
                    position.x + node_width / 2.0,
                    position.y + node_height / 2.0 + 12.0,
                ),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center,
                color: self.secondary_text_color(),
                size: Pixels(10.0),
                ..canvas::Text::default()
            });
        }
    }

    /// Draw a pulsing glow effect for running nodes
    fn draw_running_glow(&self, frame: &mut canvas::Frame, origin: Point, size: Size) {
        // Pulse between 0.1 and 0.3 alpha
        let pulse = (self.animation_frame as f32 * 0.1).sin() * 0.1 + 0.2;
        let glow_color = Color::from_rgba(0.4, 0.7, 1.0, pulse);

        // Draw outer glow
        let glow_rect = canvas::Path::rounded_rectangle(
            Point::new(origin.x - 3.0, origin.y - 3.0),
            Size::new(size.width + 6.0, size.height + 6.0),
            12.0.into(),
        );
        frame.fill(&glow_rect, glow_color);
    }
}

impl<Message> canvas::Program<Message> for FlowCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let mut node_width = 180.0;
        let node_height = 48.0;
        let mut h_gap = 40.0;
        let v_gap = 24.0;
        let margin = 16.0;

        let mut depth = std::collections::HashMap::new();
        depth.insert(self.graph.root_id(), 0usize);

        let mut changed = true;
        while changed {
            changed = false;
            for edge in self.graph.edges() {
                if let Some(parent_depth) = depth.get(&edge.from).copied() {
                    let next_depth = parent_depth + 1;
                    let entry = depth.entry(edge.to).or_insert(next_depth);
                    if *entry < next_depth {
                        *entry = next_depth;
                        changed = true;
                    }
                }
            }
        }

        let max_depth = depth.values().copied().max().unwrap_or(0);
        let columns = (max_depth + 1).max(1) as f32;
        let available_width = (bounds.width - margin * 2.0).max(120.0);
        let total_width = columns * node_width + (columns - 1.0) * h_gap;
        let scale = (available_width / total_width).clamp(0.6, 1.0);
        node_width *= scale;
        h_gap *= scale;

        let mut positions: std::collections::HashMap<usize, Point> =
            std::collections::HashMap::new();
        for (index, node_id) in self.graph.order().iter().enumerate() {
            if let Some(node) = self.graph.nodes().get(node_id) {
                let node_depth = depth.get(node_id).copied().unwrap_or(0);
                let x = margin + (node_depth as f32) * (node_width + h_gap);
                let y = margin + (index as f32) * (node_height + v_gap);
                positions.insert(node.id, Point::new(x, y));
            }
        }

        // Draw grid dots
        let grid_color = self.grid_color();
        let step = 20.0;
        let bounds_size = bounds.size();
        let max_x = bounds_size.width;
        let max_y = bounds_size.height;
        let mut y = 0.0;
        while y < max_y {
            let mut x = 0.0;
            while x < max_x {
                let dot = canvas::Path::circle(Point::new(x, y), 1.0);
                frame.fill(&dot, grid_color);
                x += step;
            }
            y += step;
        }

        if self.graph.runs().is_empty() {
            frame.fill_text(canvas::Text {
                content: "Waiting for agent...".to_string(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center,
                color: self.secondary_text_color(),
                size: Pixels(14.0),
                ..canvas::Text::default()
            });

            return vec![frame.into_geometry()];
        }

        // Draw edges
        for edge in self.graph.edges() {
            if let (Some(from), Some(to)) = (positions.get(&edge.from), positions.get(&edge.to)) {
                let start = Point::new(from.x + node_width, from.y + node_height / 2.0);
                let end = Point::new(to.x, to.y + node_height / 2.0);
                let mid_x = (start.x + end.x) / 2.0;
                let path = canvas::Path::new(|builder| {
                    builder.move_to(start);
                    builder.bezier_curve_to(
                        Point::new(mid_x, start.y),
                        Point::new(mid_x, end.y),
                        end,
                    );
                });
                let glow = Color {
                    a: 0.25,
                    ..self.edge_color()
                };
                frame.stroke(
                    &path,
                    canvas::Stroke::default().with_width(6.0).with_color(glow),
                );
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(2.0)
                        .with_color(self.edge_color()),
                );
            }
        }

        // Draw nodes
        for node_id in self.graph.order() {
            if let Some(node) = self.graph.nodes().get(node_id) {
                let Some(position) = positions.get(&node.id) else {
                    continue;
                };
                let origin = Point::new(position.x, position.y);
                let size = Size::new(node_width, node_height);

                // Draw pulsing glow for running nodes
                if node.status == AgentStatus::Running {
                    self.draw_running_glow(&mut frame, origin, size);
                }

                let rounded = canvas::Path::rounded_rectangle(origin, size, 10.0.into());
                let shadow = canvas::Path::rounded_rectangle(
                    Point::new(origin.x + 2.0, origin.y + 3.0),
                    size,
                    10.0.into(),
                );
                frame.fill(&shadow, Color::from_rgba(0.0, 0.0, 0.0, 0.2));
                frame.fill(&rounded, self.node_color(node));

                // Border color varies by status
                let border_color = match node.status {
                    AgentStatus::Running => Color::from_rgba(0.5, 0.7, 1.0, 0.8),
                    AgentStatus::Completed => Color {
                        a: 0.4,
                        ..self.edge_color()
                    },
                    AgentStatus::Failed => Color::from_rgba(0.9, 0.3, 0.3, 0.8),
                };
                frame.stroke(
                    &rounded,
                    canvas::Stroke::default()
                        .with_width(1.5)
                        .with_color(border_color),
                );

                // Draw node label
                frame.fill_text(canvas::Text {
                    content: node.label.clone(),
                    position: Point::new(
                        position.x + node_width / 2.0,
                        position.y + node_height / 2.0 - 4.0,
                    ),
                    align_x: alignment::Horizontal::Center.into(),
                    align_y: alignment::Vertical::Center,
                    color: self.text_color(),
                    size: Pixels(14.0),
                    ..canvas::Text::default()
                });

                // Draw status indicator
                self.draw_status_indicator(&mut frame, *position, node_width, node.status);

                // Draw duration/elapsed text
                self.draw_duration_text(&mut frame, node, *position, node_width, node_height);
            }
        }

        vec![frame.into_geometry()]
    }
}
