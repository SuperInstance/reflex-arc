//! # reflex-arc — Reflex Arc Pattern for Agent Cognition
//!
//! Like a spinal reflex, some agent responses should bypass higher cognition.
//! Stimulus → threshold check → response. Fast, deterministic, no deliberation.

use std::collections::HashMap;

// ─── Stimulus ────────────────────────────────────────────────────────────────

/// A stimulus that triggers a reflex.
#[derive(Debug, Clone)]
pub struct Stimulus {
    pub kind: String,
    pub intensity: f64,
    pub source: String,
    pub metadata: HashMap<String, String>,
}

impl Stimulus {
    pub fn new(kind: &str, intensity: f64) -> Self {
        Self { kind: kind.to_string(), intensity, source: String::new(), metadata: HashMap::new() }
    }

    pub fn from_source(kind: &str, intensity: f64, source: &str) -> Self {
        Self { kind: kind.to_string(), intensity, source: source.to_string(), metadata: HashMap::new() }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

// ─── Response ────────────────────────────────────────────────────────────────

/// A reflex response action.
#[derive(Debug, Clone)]
pub struct Response {
    pub action: String,
    pub urgency: Urgency,
    pub params: HashMap<String, f64>,
}

impl Response {
    pub fn new(action: &str, urgency: Urgency) -> Self {
        Self { action: action.to_string(), urgency, params: HashMap::new() }
    }

    pub fn with_param(mut self, key: &str, value: f64) -> Self {
        self.params.insert(key.to_string(), value);
        self
    }
}

/// Response urgency levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

// ─── ReflexRule ──────────────────────────────────────────────────────────────

/// A single reflex rule: stimulus kind → threshold → response.
#[derive(Debug, Clone)]
pub struct ReflexRule {
    pub stimulus_kind: String,
    pub threshold: f64,
    pub response_action: String,
    pub response_urgency: Urgency,
    pub cooldown_ms: u64,
}

impl ReflexRule {
    pub fn new(stimulus_kind: &str, threshold: f64, action: &str, urgency: Urgency) -> Self {
        Self {
            stimulus_kind: stimulus_kind.to_string(),
            threshold,
            response_action: action.to_string(),
            response_urgency: urgency,
            cooldown_ms: 0,
        }
    }

    pub fn with_cooldown(mut self, ms: u64) -> Self {
        self.cooldown_ms = ms;
        self
    }

    /// Does this stimulus trigger the rule?
    pub fn triggers(&self, stimulus: &Stimulus) -> bool {
        stimulus.kind == self.stimulus_kind && stimulus.intensity >= self.threshold
    }
}

// ─── ReflexArc ───────────────────────────────────────────────────────────────

/// A reflex arc — a collection of rules that process stimuli into responses.
#[derive(Debug, Clone)]
pub struct ReflexArc {
    rules: Vec<ReflexRule>,
    last_fire: HashMap<String, u64>,
    stats: ReflexStats,
}

#[derive(Debug, Clone, Default)]
pub struct ReflexStats {
    pub stimuli_processed: u64,
    pub reflexes_fired: u64,
    pub reflexes_suppressed: u64,
    pub by_kind: HashMap<String, u64>,
}

impl ReflexArc {
    pub fn new() -> Self {
        Self { rules: Vec::new(), last_fire: HashMap::new(), stats: ReflexStats::default() }
    }

    /// Add a reflex rule.
    pub fn add_rule(&mut self, rule: ReflexRule) {
        self.rules.push(rule);
    }

    /// Process a stimulus, returning responses that fired.
    pub fn process(&mut self, stimulus: &Stimulus, now_ms: u64) -> Vec<Response> {
        let mut responses = Vec::new();
        self.stats.stimuli_processed += 1;

        for rule in &self.rules {
            if rule.triggers(stimulus) {
                // Check cooldown
                let key = format!("{}:{}", rule.stimulus_kind, rule.response_action);
                if let Some(&last) = self.last_fire.get(&key) {
                    if now_ms.saturating_sub(last) < rule.cooldown_ms {
                        self.stats.reflexes_suppressed += 1;
                        continue;
                    }
                }

                self.last_fire.insert(key, now_ms);
                *self.stats.by_kind.entry(stimulus.kind.clone()).or_insert(0) += 1;
                self.stats.reflexes_fired += 1;

                responses.push(Response::new(&rule.response_action, rule.response_urgency));
            }
        }

        responses.sort_by(|a, b| b.urgency.cmp(&a.urgency));
        responses
    }

    /// Number of rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get stats.
    pub fn stats(&self) -> &ReflexStats {
        &self.stats
    }

    /// Reset stats.
    pub fn reset_stats(&mut self) {
        self.stats = ReflexStats::default();
    }
}

impl Default for ReflexArc {
    fn default() -> Self { Self::new() }
}

// ─── Multi-Arc Reflex System ─────────────────────────────────────────────────

/// A reflex system with multiple named arcs.
#[derive(Debug)]
pub struct ReflexSystem {
    arcs: HashMap<String, ReflexArc>,
}

impl ReflexSystem {
    pub fn new() -> Self {
        Self { arcs: HashMap::new() }
    }

    pub fn add_arc(&mut self, name: &str, arc: ReflexArc) {
        self.arcs.insert(name.to_string(), arc);
    }

    /// Process stimulus through a named arc.
    pub fn process(&mut self, arc_name: &str, stimulus: &Stimulus, now_ms: u64) -> Option<Vec<Response>> {
        self.arcs.get_mut(arc_name).map(|arc| arc.process(stimulus, now_ms))
    }

    /// Process through ALL arcs, collecting all responses.
    pub fn process_all(&mut self, stimulus: &Stimulus, now_ms: u64) -> Vec<Response> {
        let mut all = Vec::new();
        for arc in self.arcs.values_mut() {
            all.extend(arc.process(stimulus, now_ms));
        }
        all.sort_by(|a, b| b.urgency.cmp(&a.urgency));
        all
    }

    pub fn arc_count(&self) -> usize {
        self.arcs.len()
    }
}

impl Default for ReflexSystem {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stimulus_creation() {
        let s = Stimulus::new("heat", 0.8).with_metadata("location", "engine");
        assert_eq!(s.kind, "heat");
        assert!((s.intensity - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_response_urgency_ordering() {
        assert!(Urgency::Critical > Urgency::High);
        assert!(Urgency::High > Urgency::Medium);
        assert!(Urgency::Medium > Urgency::Low);
    }

    #[test]
    fn test_rule_triggers() {
        let rule = ReflexRule::new("heat", 0.7, "cool_down", Urgency::High);
        let high = Stimulus::new("heat", 0.9);
        let low = Stimulus::new("heat", 0.3);
        let wrong = Stimulus::new("cold", 0.9);
        assert!(rule.triggers(&high));
        assert!(!rule.triggers(&low));
        assert!(!rule.triggers(&wrong));
    }

    #[test]
    fn test_reflex_arc_basic() {
        let mut arc = ReflexArc::new();
        arc.add_rule(ReflexRule::new("heat", 0.7, "cool_down", Urgency::High));
        let resp = arc.process(&Stimulus::new("heat", 0.9), 0);
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].action, "cool_down");
    }

    #[test]
    fn test_reflex_arc_below_threshold() {
        let mut arc = ReflexArc::new();
        arc.add_rule(ReflexRule::new("heat", 0.7, "cool_down", Urgency::High));
        let resp = arc.process(&Stimulus::new("heat", 0.3), 0);
        assert!(resp.is_empty());
    }

    #[test]
    fn test_reflex_arc_cooldown() {
        let mut arc = ReflexArc::new();
        arc.add_rule(ReflexRule::new("heat", 0.5, "cool", Urgency::Medium).with_cooldown(1000));

        let r1 = arc.process(&Stimulus::new("heat", 0.8), 0);
        assert_eq!(r1.len(), 1);

        let r2 = arc.process(&Stimulus::new("heat", 0.8), 500);
        assert!(r2.is_empty()); // cooldown

        let r3 = arc.process(&Stimulus::new("heat", 0.8), 1001);
        assert_eq!(r3.len(), 1); // cooldown expired
    }

    #[test]
    fn test_reflex_arc_stats() {
        let mut arc = ReflexArc::new();
        arc.add_rule(ReflexRule::new("a", 0.5, "x", Urgency::Low));
        arc.process(&Stimulus::new("a", 0.6), 0);
        arc.process(&Stimulus::new("a", 0.3), 0);
        arc.process(&Stimulus::new("a", 0.8), 0);
        let stats = arc.stats();
        assert_eq!(stats.stimuli_processed, 3);
        assert_eq!(stats.reflexes_fired, 2);
    }

    #[test]
    fn test_reflex_arc_priority_sorting() {
        let mut arc = ReflexArc::new();
        arc.add_rule(ReflexRule::new("x", 0.5, "low", Urgency::Low));
        arc.add_rule(ReflexRule::new("x", 0.5, "crit", Urgency::Critical));
        let resp = arc.process(&Stimulus::new("x", 0.8), 0);
        assert_eq!(resp[0].action, "crit"); // critical first
    }

    #[test]
    fn test_reflex_system() {
        let mut sys = ReflexSystem::new();
        let mut thermal = ReflexArc::new();
        thermal.add_rule(ReflexRule::new("heat", 0.7, "cool", Urgency::High));
        sys.add_arc("thermal", thermal);

        let resp = sys.process("thermal", &Stimulus::new("heat", 0.9), 0).unwrap();
        assert_eq!(resp.len(), 1);
    }

    #[test]
    fn test_reflex_system_process_all() {
        let mut sys = ReflexSystem::new();
        let mut arc1 = ReflexArc::new();
        arc1.add_rule(ReflexRule::new("alert", 0.5, "wake", Urgency::Critical));
        let mut arc2 = ReflexArc::new();
        arc2.add_rule(ReflexRule::new("alert", 0.5, "log", Urgency::Low));
        sys.add_arc("safety", arc1);
        sys.add_arc("logging", arc2);

        let resp = sys.process_all(&Stimulus::new("alert", 0.8), 0);
        assert_eq!(resp.len(), 2);
        assert_eq!(resp[0].urgency, Urgency::Critical);
    }

    #[test]
    fn test_response_with_params() {
        let r = Response::new("throttle", Urgency::Medium).with_param("rate", 0.5);
        assert!((r.params["rate"] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_reset_stats() {
        let mut arc = ReflexArc::new();
        arc.add_rule(ReflexRule::new("x", 0.1, "y", Urgency::Low));
        arc.process(&Stimulus::new("x", 0.5), 0);
        assert_eq!(arc.stats().reflexes_fired, 1);
        arc.reset_stats();
        assert_eq!(arc.stats().reflexes_fired, 0);
    }
}
