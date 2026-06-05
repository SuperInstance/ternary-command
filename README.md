# ternary-command: Command parsing and dispatch with ternary outcomes

Parse raw text into structured commands, register handlers, and dispatch with results that are always one of three states: **Success**, **Partial**, or **Failure**.

## Why This Exists

In multi-agent systems, commands don't always succeed or fail cleanly. An agent might partially execute an order (e.g., moved but couldn't complete the action). Binary success/failure loses information. Ternary outcomes capture the middle ground, giving supervisors and audit systems the fidelity they need.

## Core Concepts

- **CommandResult**: A ternary enum — `Success(msg)`, `Partial(msg)`, or `Failure(msg)`. Every command dispatch returns one.
- **CommandParser**: Splits raw text into a verb + positional args. Configurable delimiter.
- **CommandRegistry**: Maps verb strings to handler functions. Look up and dispatch.
- **CommandContext**: Tracks who issued the command, where, and when (timestamp in ms since epoch).
- **CommandHistory**: Append-only audit trail with capacity limits, filtering by agent or result type.
- **AliasSystem**: Expands short aliases into full command strings before parsing.

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-command = "0.1"
```

```rust
use ternary_command::*;

fn handler(cmd: &ParsedCommand, _ctx: &CommandContext) -> CommandResult {
    if cmd.args.is_empty() {
        CommandResult::Failure("needs an argument".into())
    } else {
        CommandResult::Success(format!("did {} with {:?}", cmd.verb, cmd.args))
    }
}

fn main() {
    let parser = CommandParser::new();
    let mut registry = CommandRegistry::new();
    registry.register("go", handler);

    let parsed = parser.parse("go north").unwrap();
    let ctx = CommandContext::new("agent-1", "room-42");
    let result = registry.get(&parsed.verb).unwrap()(&parsed, &ctx);
    assert!(result.is_success());
}
```

## API Overview

| Type | Description |
|------|-------------|
| `CommandResult` | Ternary outcome: Success, Partial, or Failure with a message |
| `CommandContext` | Who/where/when metadata for a command invocation |
| `ParsedCommand` | Structured command: verb, args, and original raw text |
| `CommandParser` | Splits raw text into a `ParsedCommand` |
| `CommandRegistry` | Maps verb strings to handler functions |
| `CommandHistory` | Append-only audit trail with capacity and filtering |
| `AliasSystem` | Expand short aliases into full command strings |

## How It Works

`CommandParser` splits input on a configurable delimiter (default: space), treating the first token as the verb and the rest as positional arguments. Empty tokens from extra whitespace are discarded.

`CommandRegistry` stores handler functions (`fn(&ParsedCommand, &CommandContext) -> CommandResult`) in a `HashMap`. Dispatch is a simple lookup + call — no middleware, no async.

`CommandHistory` is an append-only `Vec` with an optional capacity cap. When the cap is reached, the oldest entry is evicted (FIFO). Entries can be filtered by agent ID or result type.

`AliasSystem` matches only the first token of input against registered aliases. If matched, the token is replaced with the expansion; any trailing text is preserved.

## Known Limitations

- **No quoted arguments**: The parser splits purely on the delimiter. `"say hello world"` will produce three tokens, not one. A future version should support quoted strings.
- **No async support**: Handlers are synchronous function pointers. Async runtimes are out of scope for now.
- **Handler type is a function pointer**: You can't use closures as handlers (they don't implement `fn`). This limits dynamic dispatch patterns.
- **History eviction is O(n)**: When capacity is full, removing the oldest entry shifts the entire Vec. Fine for small histories; use `with_capacity` thoughtfully.
- **No command validation**: The registry doesn't check arg counts or types before calling the handler. That's the handler's job.

## Use Cases

- **Game engine CLI**: Player types commands like `move north`, `give sword knight`. Ternary results handle partial success (e.g., moved but hit a wall).
- **Agent tasking system**: Supervisor dispatches orders to agents. `Partial` means the agent started but needs more resources. `Failure` means it couldn't start.
- **Chat bot command handler**: Slash commands in a chat system. Aliases let users type `/n` instead of `/move north`.
- **DevOps runbook automation**: Operators type commands, history provides audit trail, registry ensures only approved verbs are available.

## Ecosystem Context

Part of the **SuperInstance** ternary crate family. Relates to:

- **ternary-event**: Where commands produce events that need pub/sub distribution
- **ternary-trust**: Where repeated command failures may affect trust scores
- **ternary-kalman**: Where command outcomes feed into state estimation

This crate is a leaf dependency — it doesn't depend on other ternary crates.

## License

MIT

## See Also
- **ternary-cli** — related
- **ternary-engine** — related
- **ternary-protocol** — related
- **ternary-compiler** — related
- **ternary-grammar** — related

