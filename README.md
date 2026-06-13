# Ternary Command

**Ternary Command** provides command parsing and dispatch with ternary outcomes — every command resolves to Success (+1), Partial (0), or Failure (-1), integrating the ternary philosophy into the fleet's command interface.

## Why It Matters

Command systems usually return binary success/failure, leaving no room for "it partially worked." But fleet operations frequently produce partial results: a sensor scan that gets 70% of readings, a deployment that activates 3 of 4 nodes, or a cache refresh that updates most keys. Ternary Command's three-valued outcome captures this nuance, enabling more sophisticated error handling — Partial results can be used with degraded functionality rather than discarded entirely.

## How It Works

### Command Result

```rust
enum CommandResult {
    Success(String),   // +1: fully completed
    Partial(String),   //  0: partially completed
    Failure(String),   // -1: failed
}
```

Each variant carries a message payload. Helper methods: `is_success()`, `is_partial()`, `is_failure()`, `message()`.

### Command Context

```rust
CommandContext {
    agent_id: String,     // who issued the command
    location: String,     // where it was issued
    timestamp_ms: u64,    // when (epoch milliseconds)
}
```

Context provides audit trail information. Creation with current time: **O(1)**.

### Registry and Dispatch

```rust
CommandRegistry {
    commands: HashMap<String, CommandHandler>,
    aliases: HashMap<String, String>,       // alias → canonical name
    history: Vec<(CommandContext, CommandResult)>,
}
```

- `register(name, handler)` → **O(1)** HashMap insert
- `dispatch(name, args, context)` → **O(1)** lookup + handler execution
- Alias resolution: **O(1)** (check aliases HashMap, fall through to canonical)

### History Tracking

Every dispatched command is logged:

```
history.push((context, result))
```

History append: **O(1)** amortized. History query by agent: **O(N)** scan. Configurable max history size with FIFO eviction.

### Alias System

```rust
registry.alias("ls", "list")
registry.alias("exit", "quit")
```

Aliases resolve transparently — dispatching "ls" invokes the "list" handler.

## Quick Start

```rust
use ternary_command::{CommandRegistry, CommandResult, CommandContext};

let mut registry = CommandRegistry::new();
registry.register("scan", |_args, _ctx| {
    CommandResult::Success("Scan complete".into())
});
registry.register("partial_scan", |_args, _ctx| {
    CommandResult::Partial("70% coverage".into())
});

let ctx = CommandContext::new("agent-1", "bridge");
let result = registry.dispatch("scan", &["--full"], &ctx);
assert!(result.is_success());
```

## API

| Type | Description |
|------|-------------|
| `CommandResult` | Success(String), Partial(String), Failure(String) |
| `CommandContext` | agent_id, location, timestamp_ms |
| `CommandRegistry` | Register, dispatch, alias, history |
| `CommandHandler` | `fn(args: &[String], ctx: &CommandContext) -> CommandResult` |

Key methods: `register()`, `dispatch()`, `alias()`, `history()`.

## Architecture Notes

Ternary Command provides the command interface for fleet operations in SuperInstance. In γ + η = C, Success (+1) represents γ (growth — command fully achieved its objective), Failure (-1) represents η (avoidance — command failed, avoid the result), and Partial (0) is the neutral state where some progress was made but the objective wasn't fully met. Integrates with `ternary-captain` for leadership commands and `ternary-channel` for remote command dispatch.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for command architecture.


### Alias System

Aliases provide human-friendly shortcuts:

```rust
registry.alias("ls", "list")
registry.alias("exit", "quit")
registry.alias("scan", "sensor_scan_full")
```

Alias resolution: **O(1)** HashMap lookup. Aliases are transparent — dispatching "ls" invokes the "list" handler with all arguments forwarded. An alias can shadow another alias (chains resolve in **O(K)** for K chained aliases, with cycle detection).

### History and Audit Trail

Every dispatched command is logged with full context:

```
history: Vec<(CommandContext, CommandResult)>

history query by agent_id: O(N) scan → Vec<(context, result)>
history query by time range: O(N) scan with timestamp filter
```

Configurable max history size (default 10,000 entries) with FIFO eviction. The audit trail enables: replay (reconstruct fleet state from command sequence), accountability (who issued what when), and debugging (what command caused the anomaly).

## References

1. Gamma, E. et al. (1994). *Design Patterns*. Addison-Wesley. Command Pattern.
2. Schedulers and Dispatchers. Linux Kernel Documentation, 2024.
3. Bustamante, D. (2019). "Command Query Responsibility Segregation (CQRS)." *Microsoft Azure Architecture Center*.

## License

MIT
