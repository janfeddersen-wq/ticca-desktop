//! Smooth scrolling component with inertia physics.
//!
//! Provides buttery-smooth 120FPS scrolling with momentum,
//! friction-based deceleration, and snap-to-target animation.

use std::time::Instant;

/// Smooth scrolling state with physics-based animation.
///
/// Tracks position, velocity, and target for inertia scrolling.
/// Designed for 120FPS performance with minimal computation per frame.
#[derive(Debug, Clone)]
pub struct SmoothScroll {
    /// Current scroll position (pixels from top)
    position: f32,
    /// Current velocity (pixels per second)
    velocity: f32,
    /// Optional target position for animated scroll-to
    target: Option<f32>,
    /// Friction coefficient (0.0-1.0, higher = more friction)
    friction: f32,
    /// Maximum content height
    content_height: f32,
    /// Viewport height
    viewport_height: f32,
    /// Last update timestamp
    last_update: Instant,
    /// Whether animation is currently active
    is_animating: bool,
    /// Minimum velocity threshold (stop animation below this)
    velocity_threshold: f32,
    /// Spring stiffness for scroll-to animation
    spring_stiffness: f32,
    /// Spring damping for scroll-to animation
    spring_damping: f32,
}

impl Default for SmoothScroll {
    fn default() -> Self {
        Self::new(0.92)
    }
}

impl SmoothScroll {
    /// Create a new smooth scroll instance with specified friction.
    ///
    /// # Arguments
    /// * `friction` - Friction coefficient (0.0-1.0). Higher values mean
    ///   more friction and faster stopping. Recommended: 0.90-0.95
    pub fn new(friction: f32) -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            target: None,
            friction: friction.clamp(0.0, 0.99),
            content_height: 0.0,
            viewport_height: 0.0,
            last_update: Instant::now(),
            is_animating: false,
            velocity_threshold: 0.5,
            spring_stiffness: 300.0,
            spring_damping: 25.0,
        }
    }

    /// Set the content and viewport dimensions.
    pub fn set_dimensions(&mut self, content_height: f32, viewport_height: f32) {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
        // Clamp position to valid range
        self.position = self.clamp_position(self.position);
    }

    /// Get the current scroll position.
    pub fn position(&self) -> f32 {
        self.position
    }

    /// Get the current velocity.
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Check if currently animating.
    pub fn is_animating(&self) -> bool {
        self.is_animating
    }

    /// Get maximum scroll position.
    pub fn max_scroll(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    /// Clamp a position to valid scroll range.
    fn clamp_position(&self, pos: f32) -> f32 {
        pos.clamp(0.0, self.max_scroll())
    }

    /// Apply a scroll delta (from mouse wheel or touch).
    ///
    /// Adds velocity based on delta, enabling momentum scrolling.
    pub fn scroll_by(&mut self, delta: f32) {
        // Cancel any target animation
        self.target = None;

        // Add to velocity (convert delta to velocity)
        self.velocity += delta * 15.0; // Scale factor for natural feel

        // Clamp velocity to reasonable bounds
        self.velocity = self.velocity.clamp(-5000.0, 5000.0);

        self.is_animating = true;
        self.last_update = Instant::now();
    }

    /// Scroll to an absolute position with animation.
    pub fn scroll_to(&mut self, position: f32) {
        let clamped = self.clamp_position(position);
        self.target = Some(clamped);
        self.is_animating = true;
        self.last_update = Instant::now();
    }

    /// Scroll to the bottom of content.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_to(self.max_scroll());
    }

    /// Scroll to the top of content.
    pub fn scroll_to_top(&mut self) {
        self.scroll_to(0.0);
    }

    /// Instantly set position without animation.
    pub fn set_position(&mut self, position: f32) {
        self.position = self.clamp_position(position);
        self.velocity = 0.0;
        self.target = None;
        self.is_animating = false;
    }

    /// Update the scroll physics.
    ///
    /// Should be called every frame. Returns true if still animating
    /// (caller should request another frame).
    pub fn update(&mut self) -> bool {
        if !self.is_animating {
            return false;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        // Cap dt to prevent large jumps
        let dt = dt.min(0.1);

        if let Some(target) = self.target {
            // Spring-based scroll-to animation
            let displacement = target - self.position;

            // Spring force: F = -kx - cv (stiffness * displacement - damping * velocity)
            let spring_force = self.spring_stiffness * displacement;
            let damping_force = self.spring_damping * self.velocity;

            let acceleration = spring_force - damping_force;
            self.velocity += acceleration * dt;
            self.position += self.velocity * dt;

            // Check if we've arrived
            if displacement.abs() < 1.0 && self.velocity.abs() < self.velocity_threshold {
                self.position = target;
                self.velocity = 0.0;
                self.target = None;
                self.is_animating = false;
                return false;
            }
        } else {
            // Friction-based momentum scrolling
            self.position += self.velocity * dt;

            // Apply friction
            let friction_factor = self.friction.powf(dt * 60.0); // Normalize to 60fps
            self.velocity *= friction_factor;

            // Clamp position
            let old_position = self.position;
            self.position = self.clamp_position(self.position);

            // If we hit bounds, kill velocity
            if (self.position - old_position).abs() > 0.01 {
                self.velocity = 0.0;
            }

            // Stop if velocity is very low
            if self.velocity.abs() < self.velocity_threshold {
                self.velocity = 0.0;
                self.is_animating = false;
                return false;
            }
        }

        true
    }

    /// Force stop all animation.
    pub fn stop(&mut self) {
        self.velocity = 0.0;
        self.target = None;
        self.is_animating = false;
    }

    /// Check if scrolled to bottom (within threshold).
    pub fn is_at_bottom(&self, threshold: f32) -> bool {
        self.position >= self.max_scroll() - threshold
    }

    /// Check if scrolled to top.
    pub fn is_at_top(&self) -> bool {
        self.position <= 0.0
    }

    /// Get scroll progress (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        if self.max_scroll() <= 0.0 {
            return 0.0;
        }
        self.position / self.max_scroll()
    }
}

/// Scroll direction enum for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

impl ScrollDirection {
    /// Get direction from delta.
    pub fn from_delta(delta: f32) -> Self {
        if delta < 0.0 {
            ScrollDirection::Up
        } else {
            ScrollDirection::Down
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_scroll_creation() {
        let scroll = SmoothScroll::new(0.92);
        assert_eq!(scroll.position(), 0.0);
        assert_eq!(scroll.velocity(), 0.0);
        assert!(!scroll.is_animating());
    }

    #[test]
    fn test_set_dimensions() {
        let mut scroll = SmoothScroll::default();
        scroll.set_dimensions(1000.0, 500.0);
        assert_eq!(scroll.max_scroll(), 500.0);
    }

    #[test]
    fn test_scroll_by_starts_animation() {
        let mut scroll = SmoothScroll::default();
        scroll.set_dimensions(1000.0, 500.0);
        scroll.scroll_by(-50.0);
        assert!(scroll.is_animating());
        assert!(scroll.velocity() < 0.0); // Negative = scrolling down
    }

    #[test]
    fn test_scroll_to_sets_target() {
        let mut scroll = SmoothScroll::default();
        scroll.set_dimensions(1000.0, 500.0);
        scroll.scroll_to(250.0);
        assert!(scroll.is_animating());
    }

    #[test]
    fn test_position_clamping() {
        let mut scroll = SmoothScroll::default();
        scroll.set_dimensions(1000.0, 500.0);
        scroll.set_position(-100.0);
        assert_eq!(scroll.position(), 0.0);

        scroll.set_position(1000.0);
        assert_eq!(scroll.position(), 500.0); // max_scroll
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut scroll = SmoothScroll::default();
        scroll.set_dimensions(1000.0, 500.0);
        scroll.scroll_to_bottom();
        assert!(scroll.is_animating());
    }

    #[test]
    fn test_is_at_bottom() {
        let mut scroll = SmoothScroll::default();
        scroll.set_dimensions(1000.0, 500.0);
        scroll.set_position(500.0);
        assert!(scroll.is_at_bottom(1.0));
    }

    #[test]
    fn test_progress() {
        let mut scroll = SmoothScroll::default();
        scroll.set_dimensions(1000.0, 500.0);
        assert_eq!(scroll.progress(), 0.0);

        scroll.set_position(250.0);
        assert!((scroll.progress() - 0.5).abs() < 0.01);

        scroll.set_position(500.0);
        assert!((scroll.progress() - 1.0).abs() < 0.01);
    }
}
