# ternary-command

A command parsing, dispatch, and audit system where every command execution resolves to one of three outcomes: **Success (+1)**, **Partial (0)**, or **Failure (−1)**. Provides structured parsing, a handler registry, alias expansion, and an append-only history trail.

## Why It Matters

Standard `Result<T, E>` forces a binary classification: the command either succeeded or it didn't. But fleet agents frequently encounter situations that don't fit:

- A move command that reached the destination but took damage en route.
- A batch operation where 7 of 10 items succeeded.
- A query that returned stale data because the canonical source was unreachable.

These are **partial successes** — the agent did something useful, but the operator should be informed. Collapsing to `Ok` hides the problem; collapsing to `Err` discards the work done.

Within the **γ + η = C** framework:

| Symbol | Domain |
|--------|--------|
| γ | `CommandResult` ∈ {Success(+1), Partial(0), Failure(−1)} |
| η | Dispatch decisions: routing verbs to handlers, alias resolution |
| C | Consistency constraints: registry integrity, history invariants |

## How It Works

### Command Pipeline

```
Raw Input
    │
    ▼
┌──────────────┐
│ AliasSystem  │  "n" → "move north"
└──────┬───────┘
       │ expanded string
       ▼
┌──────────────┐
│ CommandParser │  split on delimiter, extract verb + args
└──────┬───────┘
       │ ParsedCommand { verb, args, raw }
       ▼
┌──────────────┐
│ Registry     │  verb → handler fn lookup
└──────┬───────┘
       │ handler(ParsedCommand, CommandContext)
       ▼
┌──────────────┐
│ CommandResult│  Success / Partial / Failure
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ History      │  append-only audit trail
└──────────────┘
```

### Parsing

The parser splits on a configurable delimiter (default: ASCII space), trims whitespace, and discards empty tokens:

$$\text{tokens} = \text{filter}(\text{trim}(\text{split}(s, d)), \text{non\text{-}empty})$$

The first token is the verb; remaining tokens are positional arguments.

**Complexity**: O(|s|) where |s| is the input length.

### Alias Resolution

Alias resolution replaces the **first token** of the input with its expansion if registered:

```
Input:    "g sword knight"
Alias:    "g" → "give"
Output:   "give sword knight"
```

The system preserves any arguments after the first token. Resolution is O(|first_token|) for HashMap lookup.

### History as a Ring Buffer

`CommandHistory::with_capacity(n)` uses a ring buffer: when full, the oldest entry is removed (O(n) shift due to `Vec::remove(0)`), and the new entry is appended. For production use with large histories, a `VecDeque` would provide O(1) eviction.

### Result Classification

The three outcomes map to ternary values for aggregation:

$$\text{batch\_result} = \begin{cases} +1 & \text{all results are Success} \\ -1 & \text{any result is Failure} \\ \;\;0 & \text{otherwise (mixed, no failure)} \end{cases}$$

This is a **pessimistic partial**: a single failure contaminates the batch, but all-successes is required for a positive result.

### Complexity

| Operation | Time | Notes |
|-----------|------|-------|
| `parse` | O(|s|) | Single-pass tokenization |
| `resolve` (alias) | O(1) expected | HashMap lookup + string concat |
| `register` (handler) | O(1) amortized | HashMap insert |
| `get` (handler) | O(1) expected | HashMap lookup |
| `record` (history) | O(1) or O(n) | O(1) if under capacity; O(n) if eviction needed |
| `by_agent` (history filter) | O(n) | Linear scan |
| `by_result` (history filter) | O(n) | Linear scan |

## Quick Start

```rust
use ternary_command::{
    CommandParser, CommandRegistry, CommandContext,
    CommandResult, AliasSystem, CommandHistory,
};

// Set up aliases
let mut aliases = AliasSystem::new();
aliases.register("n", "move north");
aliases.register("s", "move south");

// Parse
let parser = CommandParser::new();
let expanded = aliases.resolve("n");
let cmd = parser.parse(&expanded).unwrap();
assert_eq!(cmd.verb, "move");
assert_eq!(cmd.args, vec!["north"]);

// Register handlers
fn handle_move(cmd: &ternary_command::ParsedCommand, _ctx: &CommandContext) -> CommandResult {
    match cmd.first_arg() {
        Some("north") => CommandResult::Success("moved north".into()),
        Some(direction) => CommandResult::Partial(format!("partial: {}", direction)),
        None => CommandResult::Failure("no direction".into()),
    }
}

let mut registry = CommandRegistry::new();
registry.register("move", handle_move);

// Dispatch
let ctx = CommandContext::new("agent-1", "hub");
let handler = registry.get(&cmd.verb).unwrap();
let result = handler(&cmd, &ctx);
assert!(result.is_success());

// Record in history
let mut history = CommandHistory::new();
history.record(cmd, ctx, result);
assert_eq!(history.len(), 1);
```

## API

### `CommandResult`

```rust
pub enum CommandResult {
    Success(String),
    Partial(String),
    Failure(String),
}
```

Methods: `is_success()`, `is_partial()`, `is_failure()`, `message()`.

### `ParsedCommand`

Fields: `verb: String`, `args: Vec<String>`, `raw: String`. Methods: `first_arg()`, `arg_count()`.

### `CommandContext`

Fields: `agent_id`, `location`, `timestamp_ms`. Constructor: `new(agent_id, location)`, `with_timestamp(...)`.

### `CommandParser`

- `new()` — default space delimiter.
- `with_delimiter(char)` — custom delimiter.
- `parse(&str) -> Option<ParsedCommand>` — returns `None` on empty input.

### `CommandRegistry`

- `new()`, `register(verb, handler)`, `get(verb)`, `has(verb)`, `verbs()`, `len()`, `is_empty()`.

### `AliasSystem`

- `new()`, `register(alias, expansion)`, `remove(alias)`, `resolve(input)`, `list()`, `len()`.

### `CommandHistory`

- `new()` — unlimited capacity.
- `with_capacity(max)` — ring buffer with eviction.
- `record(cmd, ctx, result)`, `last()`, `by_agent(id)`, `by_result(success_only)`.

## Architecture Notes

The handler type is `fn(&ParsedCommand, &CommandContext) -> CommandResult` — a raw function pointer, not a closure or trait object. This keeps the registry `Send + Sync` without allocation, at the cost of preventing closures that capture state. For stateful handlers, the standard pattern is to have the function pointer read from a shared `Arc<Mutex<State>>` stored externally.

The history trail is designed for **audit**, not replay. It stores `ParsedCommand` snapshots (including the raw input string) and the `CommandContext` (agent + location + timestamp), enabling post-hoc forensic analysis: "what did agent-5 do at 14:23?"

The alias system performs **single-token expansion only**. It does not support chained aliases (alias A → alias B → command) to prevent infinite loops and keep resolution O(1).

## References

- **Fowler, M.** (2005). "Command Pattern." In *Patterns of Enterprise Application Architecture*. — Command as a first-class object.
- **Gamma, E., Helm, R., Johnson, R., & Vlissides, J.** (1994). *Design Patterns*, Ch. 5 — Command pattern for request encapsulation.
- **Russell, S., & Norvig, P.** (2020). *AI: A Modern Approach* (4th ed.), §2.3 — Agent architectures with percept-action sequences.
- **Kleene, S. C.** (1952). *Introduction to Metamathematics*. — Three-valued outcome semantics.

## License

MIT
