#![forbid(unsafe_code)]

//! ternary-command: Command parsing and dispatch with ternary outcomes.
//!
//! Provides a command parser, registry, context tracking, history, and alias
//! system. Every command resolves to one of three outcomes: Success, Partial,
//! or Failure — matching the ternary philosophy of the SuperInstance ecosystem.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// The result of executing a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// Command completed successfully.
    Success(String),
    /// Command partially completed; details in the payload.
    Partial(String),
    /// Command failed; reason in the payload.
    Failure(String),
}

impl CommandResult {
    /// Returns `true` if this is a `Success` variant.
    pub fn is_success(&self) -> bool {
        matches!(self, CommandResult::Success(_))
    }

    /// Returns `true` if this is a `Partial` variant.
    pub fn is_partial(&self) -> bool {
        matches!(self, CommandResult::Partial(_))
    }

    /// Returns `true` if this is a `Failure` variant.
    pub fn is_failure(&self) -> bool {
        matches!(self, CommandResult::Failure(_))
    }

    /// Extract the message payload regardless of variant.
    pub fn message(&self) -> &str {
        match self {
            CommandResult::Success(m) | CommandResult::Partial(m) | CommandResult::Failure(m) => m,
        }
    }
}

/// Who issued the command, where, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandContext {
    pub agent_id: String,
    pub location: String,
    pub timestamp_ms: u64,
}

impl CommandContext {
    /// Create a new context with the current system time.
    pub fn new(agent_id: impl Into<String>, location: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            agent_id: agent_id.into(),
            location: location.into(),
            timestamp_ms,
        }
    }

    /// Create a context with an explicit timestamp (useful in tests).
    pub fn with_timestamp(agent_id: impl Into<String>, location: impl Into<String>, ts: u64) -> Self {
        Self {
            agent_id: agent_id.into(),
            location: location.into(),
            timestamp_ms: ts,
        }
    }
}

/// A parsed, structured command ready for dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    pub verb: String,
    pub args: Vec<String>,
    pub raw: String,
}

impl ParsedCommand {
    /// Return the first positional argument, if any.
    pub fn first_arg(&self) -> Option<&str> {
        self.args.first().map(|s| s.as_str())
    }

    /// Return the number of positional arguments.
    pub fn arg_count(&self) -> usize {
        self.args.len()
    }
}

/// Handler function type: receives the parsed command and context, returns a result.
pub type CommandHandler = fn(&ParsedCommand, &CommandContext) -> CommandResult;

/// Stores registered verbs and their handlers.
#[derive(Debug)]
pub struct CommandRegistry {
    handlers: HashMap<String, CommandHandler>,
}

impl CommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a verb with its handler.
    pub fn register(&mut self, verb: impl Into<String>, handler: CommandHandler) {
        self.handlers.insert(verb.into(), handler);
    }

    /// Look up the handler for a verb.
    pub fn get(&self, verb: &str) -> Option<&CommandHandler> {
        self.handlers.get(verb)
    }

    /// Returns true if the verb is registered.
    pub fn has(&self, verb: &str) -> bool {
        self.handlers.contains_key(verb)
    }

    /// List all registered verbs.
    pub fn verbs(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Number of registered verbs.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Returns true if there are no registered verbs.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses raw text into a structured `ParsedCommand`.
pub struct CommandParser {
    /// The character that separates tokens (default: whitespace).
    pub delimiter: char,
}

impl CommandParser {
    /// Create a parser with the default space delimiter.
    pub fn new() -> Self {
        Self { delimiter: ' ' }
    }

    /// Create a parser with a custom delimiter.
    pub fn with_delimiter(delimiter: char) -> Self {
        Self { delimiter }
    }

    /// Parse a raw text string into a `ParsedCommand`.
    ///
    /// The first token is the verb; remaining tokens are positional arguments.
    /// Tokens are split on the delimiter and empty tokens are discarded.
    pub fn parse(&self, raw: &str) -> Option<ParsedCommand> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let tokens: Vec<String> = trimmed
            .split(self.delimiter)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if tokens.is_empty() {
            return None;
        }
        let verb = tokens[0].clone();
        let args = tokens[1..].to_vec();
        Some(ParsedCommand {
            verb,
            args,
            raw: raw.to_string(),
        })
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

/// An entry in the command history audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub command: ParsedCommand,
    pub context: CommandContext,
    pub result: CommandResult,
}

/// Append-only audit trail of executed commands.
#[derive(Debug, Clone)]
pub struct CommandHistory {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
}

impl CommandHistory {
    /// Create a history with unlimited capacity.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: usize::MAX,
        }
    }

    /// Create a history with a maximum number of entries.
    pub fn with_capacity(max: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: max,
        }
    }

    /// Record a command execution.
    pub fn record(&mut self, command: ParsedCommand, context: CommandContext, result: CommandResult) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(HistoryEntry {
            command,
            context,
            result,
        });
    }

    /// Number of recorded entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter()
    }

    /// Return the most recent entry, if any.
    pub fn last(&self) -> Option<&HistoryEntry> {
        self.entries.last()
    }

    /// Filter entries by agent ID.
    pub fn by_agent(&self, agent_id: &str) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.context.agent_id == agent_id)
            .collect()
    }

    /// Filter entries by result type.
    pub fn by_result(&self, success_only: bool) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.result.is_success() == success_only)
            .collect()
    }
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Shortcuts that expand to full command strings.
#[derive(Debug, Clone)]
pub struct AliasSystem {
    aliases: HashMap<String, String>,
}

impl AliasSystem {
    /// Create an empty alias system.
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    /// Register an alias. `alias` expands to `expansion`.
    pub fn register(&mut self, alias: impl Into<String>, expansion: impl Into<String>) {
        self.aliases.insert(alias.into(), expansion.into());
    }

    /// Remove an alias. Returns true if it existed.
    pub fn remove(&mut self, alias: &str) -> bool {
        self.aliases.remove(alias).is_some()
    }

    /// Resolve a raw input string by replacing the leading token with its
    /// expansion (if an alias matches). Returns the expanded string.
    pub fn resolve(&self, input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return input.to_string();
        }
        let first_space = trimmed.find(' ').unwrap_or(trimmed.len());
        let first_token = &trimmed[..first_space];
        if let Some(expansion) = self.aliases.get(first_token) {
            if first_space < trimmed.len() {
                format!("{}{}", expansion, &trimmed[first_space..])
            } else {
                expansion.clone()
            }
        } else {
            input.to_string()
        }
    }

    /// List all registered aliases.
    pub fn list(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    /// Number of registered aliases.
    pub fn len(&self) -> usize {
        self.aliases.len()
    }

    /// Returns true if there are no aliases.
    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }
}

impl Default for AliasSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- CommandResult tests ---

    #[test]
    fn result_success_is_success() {
        let r = CommandResult::Success("ok".into());
        assert!(r.is_success());
        assert!(!r.is_partial());
        assert!(!r.is_failure());
        assert_eq!(r.message(), "ok");
    }

    #[test]
    fn result_partial_is_partial() {
        let r = CommandResult::Partial("halfway".into());
        assert!(r.is_partial());
        assert!(!r.is_success());
        assert!(!r.is_failure());
        assert_eq!(r.message(), "halfway");
    }

    #[test]
    fn result_failure_is_failure() {
        let r = CommandResult::Failure("nope".into());
        assert!(r.is_failure());
        assert!(!r.is_success());
        assert!(!r.is_partial());
        assert_eq!(r.message(), "nope");
    }

    #[test]
    fn result_equality() {
        assert_eq!(
            CommandResult::Success("a".into()),
            CommandResult::Success("a".into())
        );
        assert_ne!(
            CommandResult::Success("a".into()),
            CommandResult::Success("b".into())
        );
    }

    // --- CommandContext tests ---

    #[test]
    fn context_new_has_nonzero_timestamp() {
        let ctx = CommandContext::new("agent-1", "hub");
        assert_eq!(ctx.agent_id, "agent-1");
        assert_eq!(ctx.location, "hub");
        assert!(ctx.timestamp_ms > 0);
    }

    #[test]
    fn context_with_timestamp_explicit() {
        let ctx = CommandContext::with_timestamp("a", "b", 42);
        assert_eq!(ctx.timestamp_ms, 42);
    }

    // --- CommandParser tests ---

    #[test]
    fn parse_simple_command() {
        let parser = CommandParser::new();
        let cmd = parser.parse("move north").unwrap();
        assert_eq!(cmd.verb, "move");
        assert_eq!(cmd.args, vec!["north"]);
        assert_eq!(cmd.raw, "move north");
    }

    #[test]
    fn parse_no_args() {
        let parser = CommandParser::new();
        let cmd = parser.parse("look").unwrap();
        assert_eq!(cmd.verb, "look");
        assert!(cmd.args.is_empty());
        assert_eq!(cmd.arg_count(), 0);
        assert!(cmd.first_arg().is_none());
    }

    #[test]
    fn parse_multiple_args() {
        let parser = CommandParser::new();
        let cmd = parser.parse("give sword knight").unwrap();
        assert_eq!(cmd.verb, "give");
        assert_eq!(cmd.args, vec!["sword", "knight"]);
        assert_eq!(cmd.arg_count(), 2);
        assert_eq!(cmd.first_arg(), Some("sword"));
    }

    #[test]
    fn parse_empty_input_returns_none() {
        let parser = CommandParser::new();
        assert!(parser.parse("").is_none());
        assert!(parser.parse("   ").is_none());
    }

    #[test]
    fn parse_extra_whitespace() {
        let parser = CommandParser::new();
        let cmd = parser.parse("  hello   world  ").unwrap();
        assert_eq!(cmd.verb, "hello");
        assert_eq!(cmd.args, vec!["world"]);
    }

    #[test]
    fn parse_custom_delimiter() {
        let parser = CommandParser::with_delimiter(':');
        let cmd = parser.parse("set:name:Alice").unwrap();
        assert_eq!(cmd.verb, "set");
        assert_eq!(cmd.args, vec!["name", "Alice"]);
    }

    // --- CommandRegistry tests ---

    fn noop_handler(_cmd: &ParsedCommand, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Success("done".into())
    }

    fn fail_handler(_cmd: &ParsedCommand, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Failure("nope".into())
    }

    #[test]
    fn register_and_lookup() {
        let mut reg = CommandRegistry::new();
        reg.register("ping", noop_handler);
        assert!(reg.has("ping"));
        assert!(!reg.has("pong"));
        assert!(reg.get("ping").is_some());
    }

    #[test]
    fn registry_list_verbs() {
        let mut reg = CommandRegistry::new();
        reg.register("a", noop_handler);
        reg.register("b", noop_handler);
        let mut verbs = reg.verbs();
        verbs.sort();
        assert_eq!(verbs, vec!["a", "b"]);
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn registry_default_is_empty() {
        let reg = CommandRegistry::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn dispatch_returns_handler_result() {
        let mut reg = CommandRegistry::new();
        reg.register("fail", fail_handler);
        let handler = reg.get("fail").unwrap();
        let cmd = ParsedCommand {
            verb: "fail".into(),
            args: vec![],
            raw: "fail".into(),
        };
        let ctx = CommandContext::with_timestamp("x", "y", 0);
        let result = handler(&cmd, &ctx);
        assert!(result.is_failure());
    }

    // --- CommandHistory tests ---

    fn make_cmd(verb: &str) -> ParsedCommand {
        ParsedCommand { verb: verb.to_string(), args: vec![], raw: verb.to_string() }
    }

    #[test]
    fn history_record_and_last() {
        let mut hist = CommandHistory::new();
        let ctx = CommandContext::with_timestamp("a", "b", 0);
        hist.record(make_cmd("test"), ctx, CommandResult::Success("ok".into()));
        assert_eq!(hist.len(), 1);
        assert_eq!(hist.last().unwrap().command.verb, "test");
    }

    #[test]
    fn history_capacity_evicts_oldest() {
        let mut hist = CommandHistory::with_capacity(2);
        let ctx = CommandContext::with_timestamp("a", "b", 0);
        let ok = CommandResult::Success("ok".into());
        hist.record(make_cmd("one"), ctx.clone(), ok.clone());
        hist.record(make_cmd("two"), ctx.clone(), ok.clone());
        hist.record(make_cmd("three"), ctx.clone(), ok.clone());
        assert_eq!(hist.len(), 2);
        assert_eq!(hist.last().unwrap().command.verb, "three");
    }

    #[test]
    fn history_filter_by_agent() {
        let mut hist = CommandHistory::new();
        let ok = CommandResult::Success("ok".into());
        hist.record(make_cmd("a"), CommandContext::with_timestamp("agent1", "loc", 0), ok.clone());
        hist.record(make_cmd("b"), CommandContext::with_timestamp("agent2", "loc", 0), ok.clone());
        hist.record(make_cmd("c"), CommandContext::with_timestamp("agent1", "loc", 0), ok.clone());
        assert_eq!(hist.by_agent("agent1").len(), 2);
    }

    #[test]
    fn history_filter_by_result() {
        let mut hist = CommandHistory::new();
        let ctx = CommandContext::with_timestamp("x", "y", 0);
        hist.record(make_cmd("a"), ctx.clone(), CommandResult::Success("ok".into()));
        hist.record(make_cmd("b"), ctx.clone(), CommandResult::Failure("err".into()));
        assert_eq!(hist.by_result(true).len(), 1);
    }

    // --- AliasSystem tests ---

    #[test]
    fn alias_resolve_simple() {
        let mut aliases = AliasSystem::new();
        aliases.register("n", "move north");
        assert_eq!(aliases.resolve("n"), "move north");
    }

    #[test]
    fn alias_resolve_with_extra_args() {
        let mut aliases = AliasSystem::new();
        aliases.register("g", "give");
        assert_eq!(aliases.resolve("g sword knight"), "give sword knight");
    }

    #[test]
    fn alias_no_match_returns_input() {
        let aliases = AliasSystem::new();
        assert_eq!(aliases.resolve("look"), "look");
    }

    #[test]
    fn alias_remove() {
        let mut aliases = AliasSystem::new();
        aliases.register("x", "explode");
        assert!(aliases.remove("x"));
        assert!(!aliases.remove("x"));
        assert_eq!(aliases.resolve("x"), "x");
    }

    #[test]
    fn alias_empty_input() {
        let aliases = AliasSystem::new();
        assert_eq!(aliases.resolve(""), "");
        assert_eq!(aliases.resolve("   "), "   ");
    }

    #[test]
    fn alias_list_and_count() {
        let mut aliases = AliasSystem::new();
        aliases.register("n", "move north");
        aliases.register("s", "move south");
        assert_eq!(aliases.len(), 2);
        assert!(!aliases.is_empty());
        assert!(aliases.list().contains_key("n"));
    }
}
