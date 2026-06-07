# reflex-arc

> **Like a spinal reflex — stimulus → threshold → response. No deliberation needed.**

[![crates.io](https://img.shields.io/crates/v/reflex-arc.svg)](https://crates.io/crates/reflex-arc)
[![docs.rs](https://docs.rs/reflex-arc/badge.svg)](https://docs.rs/reflex-arc)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![tests](https://img.shields.io/badge/tests-12-passing-green.svg)]()

Reflex arc pattern for agent cognition. Some responses should bypass higher cognition entirely — anomaly detection, safety alerts, rate limiting. This library provides fast, deterministic stimulus-response chains with threshold gating and cooldown.

---

## Why This Exists

Not every agent decision needs a language model. When an anomaly spikes, you don't deliberate — you **react**. The reflex arc pattern, borrowed from neurobiology, gives agents:

- **Sub-millisecond response** to known stimuli
- **Threshold gating** — ignore noise, act on signal
- **Cooldown** — prevent reflex loops
- **Priority sorting** — critical responses first
- **Composable systems** — multiple named reflex arcs

This is the **spinal cord** of your agent. Let the cortex think, but let reflexes react.

---

## Architecture

```
    ┌─────────────┐
    │  Stimulus   │  (kind, intensity, source)
    │  "heat" 0.9 │
    └──────┬──────┘
           │
    ┌──────▼──────────────────────────┐
    │        ReflexArc                │
    │  ┌─────────────────────────┐    │
    │  │ Rule: heat > 0.7 → cool │────┼──► Response("cool_down", High)
    │  │ Rule: heat > 0.9 → shut │────┼──► Response("emergency_shutdown", Critical)
    │  │ Rule: cold < 0.1 → warm │    │   (not triggered)
    │  └─────────────────────────┘    │
    │  + cooldown: 1000ms             │
    │  + stats tracking               │
    └─────────────────────────────────┘
```

---

## Installation

```toml
[dependencies]
reflex-arc = "0.1.0"
```

---

## Quick Start

```rust
use reflex_arc::{ReflexArc, ReflexRule, Stimulus, Urgency};

let mut arc = ReflexArc::new();

// Add reflex rules
arc.add_rule(ReflexRule::new("heat", 0.7, "cool_down", Urgency::High));
arc.add_rule(ReflexRule::new("heat", 0.95, "emergency_shutdown", Urgency::Critical));

// Process a stimulus
let responses = arc.process(&Stimulus::new("heat", 0.9), 0);
assert_eq!(responses.len(), 1);
assert_eq!(responses[0].action, "cool_down");
```

---

## Usage Examples

### Example 1: Cooldown Prevention

```rust
use reflex_arc::{ReflexArc, ReflexRule, Stimulus, Urgency};

let mut arc = ReflexArc::new();
arc.add_rule(ReflexRule::new("alert", 0.5, "notify", Urgency::Medium).with_cooldown(1000));

// First fire works
let r1 = arc.process(&Stimulus::new("alert", 0.8), 0);
assert_eq!(r1.len(), 1);

// Second fire suppressed (cooldown)
let r2 = arc.process(&Stimulus::new("alert", 0.8), 500);
assert!(r2.is_empty());

// After cooldown expires
let r3 = arc.process(&Stimulus::new("alert", 0.8), 1001);
assert_eq!(r3.len(), 1);
```

### Example 2: Multi-Arc Reflex System

```rust
use reflex_arc::{ReflexSystem, ReflexArc, ReflexRule, Stimulus, Urgency};

let mut sys = ReflexSystem::new();

// Safety reflexes
let mut safety = ReflexArc::new();
safety.add_rule(ReflexRule::new("anomaly", 0.8, "shutdown", Urgency::Critical));
sys.add_arc("safety", safety);

// Logging reflexes
let mut logging = ReflexArc::new();
logging.add_rule(ReflexRule::new("anomaly", 0.5, "log", Urgency::Low));
sys.add_arc("logging", logging);

// Process through ALL arcs
let responses = sys.process_all(&Stimulus::new("anomaly", 0.9), 0);
assert_eq!(responses.len(), 2);
assert_eq!(responses[0].action, "shutdown"); // Critical first
assert_eq!(responses[1].action, "log");      // Low second
```

### Example 3: Stats Tracking

```rust
use reflex_arc::{ReflexArc, ReflexRule, Stimulus, Urgency};

let mut arc = ReflexArc::new();
arc.add_rule(ReflexRule::new("x", 0.5, "y", Urgency::Low));

arc.process(&Stimulus::new("x", 0.6), 0); // fires
arc.process(&Stimulus::new("x", 0.3), 0); // below threshold
arc.process(&Stimulus::new("x", 0.8), 0); // fires

let stats = arc.stats();
assert_eq!(stats.stimuli_processed, 3);
assert_eq!(stats.reflexes_fired, 2);
```

---

## The Biological Metaphor

In vertebrate neurobiology, a **reflex arc** is the neural pathway that mediates a reflex action:

1. **Receptor** → detects stimulus → `Stimulus`
2. **Sensory neuron** → afferent pathway → threshold check
3. **Integration center** → spinal cord → `ReflexArc` rules
4. **Motor neuron** → efferent pathway → `Response`
5. **Effector** → muscle contracts → agent acts

The key insight: **the brain is not involved**. Reflexes are faster than cognition. Your agent should have the same architecture — reflexes for known patterns, deliberation for novel situations.

---

## API Reference

| Type | Description |
|------|-------------|
| `Stimulus` | Input event with kind, intensity, source, metadata |
| `Response` | Output action with urgency and parameters |
| `Urgency` | Low, Medium, High, Critical (ordered) |
| `ReflexRule` | Stimulus kind → threshold → response mapping |
| `ReflexArc` | Collection of rules with cooldown and stats |
| `ReflexSystem` | Multiple named arcs, process through all |

---

## Performance

```
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

- Zero dependencies
- O(n) per stimulus where n = number of rules
- Priority-sorted responses
- Cooldown uses HashMap lookup

---

## Integration with Exocortex

`reflex-arc` powers the **Cognitive Reflex Arc** in the Exocortex architecture:

- Anomaly detection → Alert → Wake → Act (bypasses deliberation)
- Rate limiting → Throttle (prevents cascade failures)
- Safety monitoring → Emergency shutdown (instant response)
- Auto-classification → Tag + Route (reflexive sorting)

---

## License

MIT © [SuperInstance](https://github.com/SuperInstance)

---

*Part of the [Exocortex](https://github.com/SuperInstance/exocortex) project.*
