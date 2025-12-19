//! Agent call graph view (React Flow-style panel).

use iced::widget::{canvas, column, container, scrollable, text};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Pixels};

use crate::agent_graph::{AgentCallGraph, AgentNode};
use crate::messages::Message;
use crate::theme::{styles, AppTheme};

pub fn view<'a>(graph: &AgentCallGraph, theme: AppTheme) -> Element<'a, Message> {
    let header = text("Agents Flow").size(14);

    let canvas = canvas::Canvas::new(FlowCanvas::new(graph.clone(), theme))
        .width(Length::Fill)
        .height(Length::Fill);

    let content = column![
        header,
        scrollable(canvas).height(Length::Fill),
    ]
    .spacing(8)
    .padding(12);

    container(content)
        .width(Length::Fixed(280.0))
        .height(Length::Fill)
        .style(styles::flow_panel_container)
        .into()
}

#[derive(Debug, Clone)]
struct FlowCanvas {
    graph: AgentCallGraph,
    theme: AppTheme,
}

impl FlowCanvas {
    fn new(graph: AgentCallGraph, theme: AppTheme) -> Self {
        Self { graph, theme }
    }

    fn is_dark(&self) -> bool {
        self.theme.is_dark()
    }

    fn node_color(&self, node: &AgentNode) -> iced::Color {
        match node.agent_type {
            ticca_core::agents::AgentType::Coding => iced::Color::from_rgb(0.18, 0.55, 0.90),
            ticca_core::agents::AgentType::Planning => iced::Color::from_rgb(0.24, 0.70, 0.42),
        }
    }

    fn text_color(&self) -> iced::Color {
        if self.is_dark() {
            iced::Color::from_rgb(0.95, 0.95, 0.95)
        } else {
            iced::Color::from_rgb(0.10, 0.10, 0.10)
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

        let node_width = 180.0;
        let node_height = 48.0;
        let h_gap = 40.0;
        let v_gap = 28.0;
        let margin = 20.0;

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

        let mut positions: std::collections::HashMap<usize, Point> = std::collections::HashMap::new();
        for (index, node_id) in self.graph.order().iter().enumerate() {
            if let Some(node) = self.graph.nodes().get(node_id) {
                let node_depth = depth.get(node_id).copied().unwrap_or(0);
                let x = margin + (node_depth as f32) * (node_width + h_gap);
                let y = margin + (index as f32) * (node_height + v_gap);
                positions.insert(node.id, Point::new(x, y));
            }
        }

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

        for edge in self.graph.edges() {
            if let (Some(from), Some(to)) = (positions.get(&edge.from), positions.get(&edge.to)) {
                let start = Point::new(from.x + node_width, from.y + node_height / 2.0);
                let end = Point::new(to.x, to.y + node_height / 2.0);
                let path = canvas::Path::line(start, end);
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_width(2.0)
                        .with_color(self.edge_color()),
                );
            }
        }

        for node_id in self.graph.order() {
            if let Some(node) = self.graph.nodes().get(node_id) {
                let Some(position) = positions.get(&node.id) else {
                    continue;
                };
                let origin = Point::new(position.x, position.y);
                let size = Size::new(node_width, node_height);
                let rounded = canvas::Path::rounded_rectangle(origin, size, 8.0.into());
                frame.fill(&rounded, self.node_color(node));
                frame.stroke(
                    &rounded,
                    canvas::Stroke::default()
                        .with_width(1.0)
                        .with_color(self.edge_color()),
                );

                frame.fill_text(canvas::Text {
                    content: node.label.clone(),
                    position: Point::new(position.x + 10.0, position.y + 30.0),
                    color: self.text_color(),
                    size: Pixels(14.0),
                    ..canvas::Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}
