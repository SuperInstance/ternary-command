# ternary-command

**Command parsing, registry, history, and dispatch with ternary outcomes for the SuperInstance ecosystem.**

## Background

Command-line interfaces and agent instruction sets have traditionally been binary in their result model: a command succeeds or fails. Real-world distributed systems, however, frequently encounter partial success — a deployment that updates 8 of 10 nodes, a migration that completes but needs cleanup, or a configuration change that applies to some rooms but not others.

`ternary-command` introduces a **ternary result model**: every command resolves to `Success`, `Partial`, or `Failure`. This three-valued logic maps naturally to the SuperInstance ecosystem's balanced ternary philosophy (−1, 0, +1), where partial completion is a first-class outcome rather than an error code hack.

The crate provides a complete command infrastructure: parsing, registry-based dispatch, alias expansion, append-only history with filtering, and context tracking with timestamps.

## How It Works

### Command Lifecycle

1. **Parse** — `CommandParser` tokenizes raw text into a `ParsedCommand` (verb + positional args). Supports custom delimiters (default: whitespace).
2. **Alias expansion** — `AliasSystem` replaces the leading token with registered expansions (e.g., `n` → `move north`).
3. **Dispatch** — `CommandRegistry` maps verbs to handler functions (`fn(&ParsedCommand, &CommandContext) -> CommandResult`).
4. **Record** — `CommandHistory` logs the command, context, and result in an append-only audit trail.

### Ternary Results

```rust
pub enum CommandResult {
    Success(String),   // +1: fully completed
    Partial(String),   //  0: partially completed
    Failure(String),   // -1: failed
}
```

Each variant carries a descriptive message. The `is_success()`, `is_partial()`, and `is_failure()` predicates enable pattern-matching control flow, while `message()` extracts the payload regardless of variant.

### Context and History

`CommandContext` captures *who* issued the command (`agent_id`), *where* (`location`), and *when* (`timestamp_ms`). This context flows into the history log and handler functions, enabling:

- **Audit trails** — filter by agent (`by_agent()`) or result type (`by_result()`)
- **Debugging** — replay the full command sequence for a given room or agent
- **Capacity-bounded history** — `CommandHistory::with_capacity(n)` evicts oldest entries when full

### Alias System

The `AliasSystem` provides lightweight macro expansion: register `n` → `move north`, and `n fast` expands to `move north fast`. This is similar to shell aliases or Vim command abbreviations, reducing typing for frequent operations.

## Experimental Results

The test suite (20+ tests) validates:

- **Parser correctness** — simple commands, no-arg commands, multi-arg commands, extra whitespace, custom delimiters
- **Result semantics** — `Success` is success, `Failure` is failure, equality/disequality
- **Context timestamps** — auto-generated timestamps are non-zero; explicit timestamps are preserved
- **Registry dispatch** — handlers return correct results; verb lookup succeeds/fails appropriately
- **History lifecycle** — recording, capacity-based eviction, filtering by agent and result type
- **Alias expansion** — simple aliases, aliases with trailing arguments, no-match passthrough, removal

## Impact

The ternary result model has practical implications beyond philosophical elegance. In fleet management, partial success is the common case — a command that updates 95% of nodes shouldn't be treated as a failure or silently reported as success. By making `Partial` a first-class result, downstream systems can:

- Retry only the failed subset
- Surface partial-completion warnings to operators
- Build aggregate health scores from command outcomes

The command history doubles as an audit log, critical for compliance in multi-agent systems where multiple autonomous agents may issue commands concurrently.

## Use Cases

1. **Fleet orchestration** — A coordinator sends `deploy v2.3` to a fleet of rooms. Rooms that update successfully return `Success`, rooms with conflicts return `Partial` (some services updated), and rooms that fail entirely return `Failure`. The coordinator can triage based on the ternary result.

2. **Agent control plane** — Autonomous agents receive natural-language commands like `scan sector-7` or `report status`. The parser tokenizes, aliases expand shorthand, and the registry dispatches to room-specific handlers. History tracks which agent did what and when.

3. **Debugging and forensics** — When a fleet incident occurs, `CommandHistory::by_agent(agent_id)` and `by_result(false)` quickly surface the sequence of failed commands leading to the issue, with timestamps and locations for root-cause analysis.

4. **Custom REPL/CLI** — Build an interactive shell for the SuperInstance ecosystem with alias support (`ll` → `list --long`), command history, and ternary result display (✓ green for Success, ⚠ yellow for Partial, ✗ red for Failure).

5. **Audit compliance** — In regulated environments, the append-only command history serves as an audit trail showing every action taken, by whom, from where, and with what result — including partial completions that traditional binary logging would miss.

## Open Questions

- **Async handlers:** The current `CommandHandler` type is a synchronous function pointer. Should the registry support `async fn` handlers for commands that require I/O (network calls, file operations)?
- **Nested commands / pipelines:** Can the parser support command composition (e.g., `scan sector-7 | filter anomaly | alert`)? This would require a pipeline abstraction built on top of the current parser.
- **Permission model:** Currently any agent can issue any command. Should the registry integrate with a permission/capability system to restrict which agents can invoke which verbs?

## Connection to Oxide Stack

`ternary-command` is the control plane for the SuperInstance ecosystem:

- **`ternary-channel`** — commands are transported over channel abstractions
- **`ternary-event`** — command execution emits events (command.started, command.completed) for observability
- **`ternary-protocol`** — command serialization for inter-node transport
- **`ternary-voting`** — coordinated commands may require consensus before execution

The ternary result model (`Success`/`Partial`/`Failure`) mirrors the ternary values used throughout the ecosystem, ensuring consistency in how outcomes are represented and propagated.
