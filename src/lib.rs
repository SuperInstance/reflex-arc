//! # reflex-arc
//!
//! Automatic cognitive reflexes — stimulus→response mapping with priority chains,
//! monitoring (firing rates, false positives), and suppression.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

// ── Stimulus Types ──────────────────────────────────────────────────────────

/// Kinds of stimuli that can trigger reflexes.
#[derive(Debug, Clone, PartialEq)]
pub enum Stimulus {
    /// An anomaly detected (e.g. value out of range).
    Anomaly { label: String, value: f64 },
    /// A threshold crossing (value exceeded threshold).
    Threshold { metric: String, value: f64, threshold: f64 },
    /// A recognized pattern match.
    Pattern { name: String, confidence: f64 },
    /// A temporal trigger (e.g. timeout, interval elapsed).
    Temporal { event: String, elapsed_ms: u64 },
}

impl Stimulus {
    /// Confidence score for this stimulus (0.0–1.0).
    pub fn confidence(&self) -> f64 {
        match self {
            Stimulus::Anomaly { value, .. } => value.abs().min(1.0),
            Stimulus::Threshold { value, threshold, .. } => {
                if *threshold == 0.0 { 1.0 } else { (value / threshold).min(1.0) }
            }
            Stimulus::Pattern { confidence, .. } => *confidence,
            Stimulus::Temporal { .. } => 1.0,
        }
    }
}

// ── Response Types ──────────────────────────────────────────────────────────

/// Responses produced by reflexes.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Alert { message: String, severity: Severity },
    AutoCorrect { action: String, new_value: f64 },
    Log { message: String },
    Escalate { reason: String, to_level: u8 },
}

/// Severity levels for alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

// ── Reflex ──────────────────────────────────────────────────────────────────

/// A single reflex: maps a stimulus to a response if the threshold is met.
#[derive(Debug, Clone)]
pub struct Reflex {
    pub name: String,
    pub priority: i32,
    pub threshold: f64,
    pub handler: fn(&Stimulus) -> Option<Response>,
    pub enabled: bool,
}

impl Reflex {
    pub fn new(name: &str, priority: i32, threshold: f64, handler: fn(&Stimulus) -> Option<Response>) -> Self {
        Self {
            name: name.to_string(),
            priority,
            threshold,
            handler,
            enabled: true,
        }
    }

    /// Evaluate this reflex against a stimulus.
    pub fn evaluate(&self, stimulus: &Stimulus) -> Option<Response> {
        if !self.enabled {
            return None;
        }
        if stimulus.confidence() >= self.threshold {
            (self.handler)(stimulus)
        } else {
            None
        }
    }
}

// ── ReflexArc ───────────────────────────────────────────────────────────────

/// A chain of reflexes ordered by priority (highest first).
pub struct ReflexArc {
    reflexes: Vec<Reflex>,
}

impl ReflexArc {
    pub fn new() -> Self {
        Self { reflexes: Vec::new() }
    }

    /// Add a reflex to the arc.
    pub fn add(&mut self, reflex: Reflex) {
        self.reflexes.push(reflex);
        self.reflexes.sort_by_key(|b| std::cmp::Reverse(b.priority));
    }

    /// Process a stimulus through all reflexes, returning all triggered responses.
    pub fn process(&self, stimulus: &Stimulus) -> Vec<(String, Response)> {
        self.reflexes
            .iter()
            .filter_map(|r| {
                r.evaluate(stimulus).map(|resp| (r.name.clone(), resp))
            })
            .collect()
    }

    /// Process a stimulus, returning only the highest-priority response.
    pub fn process_first(&self, stimulus: &Stimulus) -> Option<(String, Response)> {
        self.reflexes.iter().find_map(|r| {
            r.evaluate(stimulus).map(|resp| (r.name.clone(), resp))
        })
    }

    pub fn reflex_count(&self) -> usize {
        self.reflexes.len()
    }

    /// Disable a reflex by name.
    pub fn disable(&mut self, name: &str) {
        if let Some(r) = self.reflexes.iter_mut().find(|r| r.name == name) {
            r.enabled = false;
        }
    }

    /// Enable a reflex by name.
    pub fn enable(&mut self, name: &str) {
        if let Some(r) = self.reflexes.iter_mut().find(|r| r.name == name) {
            r.enabled = true;
        }
    }
}

impl Default for ReflexArc {
    fn default() -> Self { Self::new() }
}

// ── ReflexMonitor ───────────────────────────────────────────────────────────

/// Tracks firing rates, false positives, and suppression for reflexes.
pub struct ReflexMonitor {
    fire_counts: HashMap<String, AtomicU64>,
    false_positive_counts: HashMap<String, AtomicU64>,
    suppressed: std::sync::Mutex<HashSet<String>>,
}

use std::collections::HashSet;

impl ReflexMonitor {
    pub fn new() -> Self {
        Self {
            fire_counts: HashMap::new(),
            false_positive_counts: HashMap::new(),
            suppressed: std::sync::Mutex::new(HashSet::new()),
        }
    }

    /// Register a reflex name for monitoring.
    pub fn register(&mut self, name: &str) {
        self.fire_counts.entry(name.to_string()).or_insert_with(|| AtomicU64::new(0));
        self.false_positive_counts.entry(name.to_string()).or_insert_with(|| AtomicU64::new(0));
    }

    /// Record a fire event for a reflex.
    pub fn record_fire(&self, name: &str) {
        if let Some(count) = self.fire_counts.get(name) {
            count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Record a false positive for a reflex.
    pub fn record_false_positive(&self, name: &str) {
        if let Some(count) = self.false_positive_counts.get(name) {
            count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Get the fire count for a reflex.
    pub fn fire_count(&self, name: &str) -> u64 {
        self.fire_counts.get(name).map(|c| c.load(Ordering::SeqCst)).unwrap_or(0)
    }

    /// Get the false positive count for a reflex.
    pub fn false_positive_count(&self, name: &str) -> u64 {
        self.false_positive_counts.get(name).map(|c| c.load(Ordering::SeqCst)).unwrap_or(0)
    }

    /// Suppress a reflex (it should not fire).
    pub fn suppress(&self, name: &str) {
        self.suppressed.lock().unwrap().insert(name.to_string());
    }

    /// Unsuppress a reflex.
    pub fn unsuppress(&self, name: &str) {
        self.suppressed.lock().unwrap().remove(name);
    }

    /// Check if a reflex is suppressed.
    pub fn is_suppressed(&self, name: &str) -> bool {
        self.suppressed.lock().unwrap().contains(name)
    }

    /// Calculate the false positive rate for a reflex.
    pub fn false_positive_rate(&self, name: &str) -> f64 {
        let fires = self.fire_count(name) as f64;
        let fps = self.false_positive_count(name) as f64;
        if fires == 0.0 { 0.0 } else { fps / fires }
    }
}

impl Default for ReflexMonitor {
    fn default() -> Self { Self::new() }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn anomaly_handler(s: &Stimulus) -> Option<Response> {
        match s {
            Stimulus::Anomaly { label, value } if value.abs() > 0.5 => {
                Some(Response::Alert {
                    message: format!("Anomaly: {} = {}", label, value),
                    severity: Severity::High,
                })
            }
            _ => None,
        }
    }

    fn threshold_handler(s: &Stimulus) -> Option<Response> {
        match s {
            Stimulus::Threshold { metric, value, threshold } if value > threshold => {
                Some(Response::AutoCorrect {
                    action: format!("Reduce {}", metric),
                    new_value: *threshold,
                })
            }
            _ => None,
        }
    }

    fn log_handler(_s: &Stimulus) -> Option<Response> {
        Some(Response::Log { message: "Noted".to_string() })
    }

    fn escalate_handler(s: &Stimulus) -> Option<Response> {
        match s {
            Stimulus::Pattern { name, confidence } if *confidence > 0.8 => {
                Some(Response::Escalate {
                    reason: format!("High confidence pattern: {}", name),
                    to_level: 3,
                })
            }
            _ => None,
        }
    }

    // ── Stimulus Tests ──

    #[test]
    fn stimulus_anomaly_confidence() {
        let s = Stimulus::Anomaly { label: "cpu".into(), value: 0.7 };
        assert!((s.confidence() - 0.7).abs() < 0.001);
    }

    #[test]
    fn stimulus_pattern_confidence() {
        let s = Stimulus::Pattern { name: "spike".into(), confidence: 0.95 };
        assert!((s.confidence() - 0.95).abs() < 0.001);
    }

    #[test]
    fn stimulus_temporal_confidence() {
        let s = Stimulus::Temporal { event: "timeout".into(), elapsed_ms: 5000 };
        assert_eq!(s.confidence(), 1.0);
    }

    // ── Reflex Tests ──

    #[test]
    fn reflex_fires_above_threshold() {
        let r = Reflex::new("anomaly", 10, 0.3, anomaly_handler);
        let s = Stimulus::Anomaly { label: "x".into(), value: 0.8 };
        assert!(r.evaluate(&s).is_some());
    }

    #[test]
    fn reflex_does_not_fire_below_threshold() {
        let r = Reflex::new("anomaly", 10, 0.9, anomaly_handler);
        let s = Stimulus::Anomaly { label: "x".into(), value: 0.3 };
        assert!(r.evaluate(&s).is_none());
    }

    #[test]
    fn reflex_disabled() {
        let mut r = Reflex::new("anomaly", 10, 0.0, anomaly_handler);
        r.enabled = false;
        let s = Stimulus::Anomaly { label: "x".into(), value: 0.9 };
        assert!(r.evaluate(&s).is_none());
    }

    // ── ReflexArc Tests ──

    #[test]
    fn arc_priority_ordering() {
        let mut arc = ReflexArc::new();
        arc.add(Reflex::new("low", 1, 0.0, log_handler));
        arc.add(Reflex::new("high", 100, 0.0, escalate_handler));
        arc.add(Reflex::new("mid", 50, 0.0, log_handler));
        // process_first should get "high" first
        let s = Stimulus::Pattern { name: "spike".into(), confidence: 0.9 };
        let result = arc.process_first(&s);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "high");
    }

    #[test]
    fn arc_process_returns_all_matches() {
        let mut arc = ReflexArc::new();
        arc.add(Reflex::new("logger", 1, 0.0, log_handler));
        arc.add(Reflex::new("escalator", 10, 0.0, escalate_handler));
        let s = Stimulus::Pattern { name: "spike".into(), confidence: 0.9 };
        let results = arc.process(&s);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn arc_disable_reflex() {
        let mut arc = ReflexArc::new();
        arc.add(Reflex::new("logger", 1, 0.0, log_handler));
        arc.disable("logger");
        let s = Stimulus::Temporal { event: "t".into(), elapsed_ms: 100 };
        let results = arc.process(&s);
        assert!(results.is_empty());
    }

    #[test]
    fn arc_enable_disabled_reflex() {
        let mut arc = ReflexArc::new();
        arc.add(Reflex::new("logger", 1, 0.0, log_handler));
        arc.disable("logger");
        arc.enable("logger");
        let s = Stimulus::Temporal { event: "t".into(), elapsed_ms: 100 };
        let results = arc.process(&s);
        assert_eq!(results.len(), 1);
    }

    // ── Monitor Tests ──

    #[test]
    fn monitor_fire_count() {
        let mut monitor = ReflexMonitor::new();
        monitor.register("test");
        monitor.record_fire("test");
        monitor.record_fire("test");
        monitor.record_fire("test");
        assert_eq!(monitor.fire_count("test"), 3);
    }

    #[test]
    fn monitor_false_positive_rate() {
        let mut monitor = ReflexMonitor::new();
        monitor.register("fp_test");
        monitor.record_fire("fp_test");
        monitor.record_fire("fp_test");
        monitor.record_false_positive("fp_test");
        let rate = monitor.false_positive_rate("fp_test");
        assert!((rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn monitor_suppression() {
        let monitor = ReflexMonitor::new();
        assert!(!monitor.is_suppressed("x"));
        monitor.suppress("x");
        assert!(monitor.is_suppressed("x"));
        monitor.unsuppress("x");
        assert!(!monitor.is_suppressed("x"));
    }

    #[test]
    fn monitor_unregistered_returns_zero() {
        let monitor = ReflexMonitor::new();
        assert_eq!(monitor.fire_count("nonexistent"), 0);
        assert_eq!(monitor.false_positive_rate("nonexistent"), 0.0);
    }
}
