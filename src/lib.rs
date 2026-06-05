#![forbid(unsafe_code)]
//! ternary-command — Command parsing and dispatch for ternary agents

use std::collections::HashMap;

/// Ternary result of a command execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    Success,   // +1
    Partial,   //  0
    Failed,    // -1
}

impl CommandResult {
    pub fn as_i8(&self) -> i8 {
        match self { Self::Success => 1, Self::Partial => 0, Self::Failed => -1 }
    }
}

/// A parsed command
#[derive(Debug, Clone)]
pub struct Command {
    pub verb: String,
    pub args: Vec<String>,
    pub raw: String,
}

/// Context in which a command is executed
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub agent_id: u64,
    pub room_id: String,
    pub tick: u64,
    pub metadata: HashMap<String, String>,
}

impl CommandContext {
    pub fn new(agent_id: u64, room_id: &str, tick: u64) -> Self {
        Self { agent_id, room_id: room_id.to_string(), tick, metadata: HashMap::new() }
    }
}

/// Handler function type for commands
pub type CommandHandler = fn(&Command, &CommandContext) -> CommandResult;

/// Registry of available commands
pub struct CommandRegistry {
    handlers: HashMap<String, CommandHandler>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new(), aliases: HashMap::new() }
    }

    pub fn register(&mut self, verb: &str, handler: CommandHandler) {
        self.handlers.insert(verb.to_lowercase(), handler);
    }

    pub fn add_alias(&mut self, alias: &str, verb: &str) {
        self.aliases.insert(alias.to_lowercase(), verb.to_lowercase());
    }

    pub fn resolve(&self, verb: &str) -> Option<(&String, &CommandHandler)> {
        let lower = verb.to_lowercase();
        if let Some(resolved) = self.aliases.get(&lower) {
            self.handlers.get_key_value(resolved)
        } else {
            self.handlers.get_key_value(&lower)
        }
    }

    pub fn has_command(&self, verb: &str) -> bool {
        self.resolve(verb).is_some()
    }

    pub fn command_count(&self) -> usize { self.handlers.len() }
    pub fn alias_count(&self) -> usize { self.aliases.len() }
}

/// Parser for text commands
pub struct CommandParser {
    delimiters: Vec<char>,
}

impl CommandParser {
    pub fn new() -> Self {
        Self { delimiters: vec![' ', '\t'] }
    }

    pub fn parse(&self, input: &str) -> Option<Command> {
        let trimmed = input.trim();
        if trimmed.is_empty() { return None; }
        let parts: Vec<&str> = trimmed.split(|c: char| self.delimiters.contains(&c))
            .filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return None; }
        Some(Command {
            verb: parts[0].to_lowercase(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
            raw: trimmed.to_string(),
        })
    }
}

/// Entry in command history
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: Command,
    pub context: CommandContext,
    pub result: CommandResult,
}

/// Audit trail of all commands
pub struct CommandHistory {
    entries: Vec<HistoryEntry>,
    max_size: usize,
}

impl CommandHistory {
    pub fn new(max_size: usize) -> Self {
        Self { entries: Vec::new(), max_size }
    }

    pub fn record(&mut self, command: Command, context: CommandContext, result: CommandResult) {
        if self.entries.len() >= self.max_size {
            self.entries.remove(0);
        }
        self.entries.push(HistoryEntry { command, context, result });
    }

    pub fn entries(&self) -> &[HistoryEntry] { &self.entries }
    pub fn len(&self) -> usize { self.entries.len() }

    pub fn success_rate(&self) -> f64 {
        if self.entries.is_empty() { return 0.0; }
        let successes = self.entries.iter().filter(|e| e.result == CommandResult::Success).count();
        successes as f64 / self.entries.len() as f64
    }

    pub fn by_agent(&self, agent_id: u64) -> Vec<&HistoryEntry> {
        self.entries.iter().filter(|e| e.context.agent_id == agent_id).collect()
    }

    pub fn by_verb(&self, verb: &str) -> Vec<&HistoryEntry> {
        self.entries.iter().filter(|e| e.command.verb == verb).collect()
    }
}

/// Dispatcher that ties parsing, registry, and history together
pub struct CommandDispatcher {
    registry: CommandRegistry,
    parser: CommandParser,
    history: CommandHistory,
}

impl CommandDispatcher {
    pub fn new(history_size: usize) -> Self {
        Self {
            registry: CommandRegistry::new(),
            parser: CommandParser::new(),
            history: CommandHistory::new(history_size),
        }
    }

    pub fn registry(&mut self) -> &mut CommandRegistry { &mut self.registry }

    pub fn dispatch(&mut self, input: &str, context: &CommandContext) -> Option<CommandResult> {
        let cmd = self.parser.parse(input)?;
        let result = if let Some((_, handler)) = self.registry.resolve(&cmd.verb) {
            handler(&cmd, context)
        } else {
            CommandResult::Failed
        };
        self.history.record(cmd, context.clone(), result);
        Some(result)
    }

    pub fn history(&self) -> &CommandHistory { &self.history }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_handler(_cmd: &Command, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Success
    }
    fn fail_handler(_cmd: &Command, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Failed
    }

    #[test] fn parse_simple() {
        let p = CommandParser::new();
        let cmd = p.parse("go north").unwrap();
        assert_eq!(cmd.verb, "go");
        assert_eq!(cmd.args, vec!["north"]);
    }

    #[test] fn parse_empty() { assert!(CommandParser::new().parse("").is_none()); }
    #[test] fn parse_whitespace() { assert!(CommandParser::new().parse("   ").is_none()); }

    #[test] fn parse_multi_args() {
        let cmd = CommandParser::new().parse("trade sword shield potion").unwrap();
        assert_eq!(cmd.verb, "trade");
        assert_eq!(cmd.args.len(), 3);
    }

    #[test] fn parse_case_insensitive() {
        let cmd = CommandParser::new().parse("GO NORTH").unwrap();
        assert_eq!(cmd.verb, "go");
    }

    #[test] fn parse_preserves_raw() {
        let cmd = CommandParser::new().parse("  Go  North  ").unwrap();
        assert_eq!(cmd.raw, "Go  North");
    }

    #[test] fn registry_register_and_resolve() {
        let mut r = CommandRegistry::new();
        r.register("go", mock_handler);
        assert!(r.has_command("go"));
        assert!(r.has_command("GO"));
        assert!(!r.has_command("stay"));
    }

    #[test] fn registry_alias() {
        let mut r = CommandRegistry::new();
        r.register("inventory", mock_handler);
        r.add_alias("inv", "inventory");
        r.add_alias("i", "inventory");
        assert!(r.has_command("inv"));
        assert!(r.has_command("i"));
    }

    #[test] fn registry_counts() {
        let mut r = CommandRegistry::new();
        r.register("go", mock_handler);
        r.register("look", mock_handler);
        r.add_alias("l", "look");
        assert_eq!(r.command_count(), 2);
        assert_eq!(r.alias_count(), 1);
    }

    #[test] fn result_values() {
        assert_eq!(CommandResult::Success.as_i8(), 1);
        assert_eq!(CommandResult::Partial.as_i8(), 0);
        assert_eq!(CommandResult::Failed.as_i8(), -1);
    }

    #[test] fn context_new() {
        let ctx = CommandContext::new(42, "tavern", 100);
        assert_eq!(ctx.agent_id, 42);
        assert_eq!(ctx.room_id, "tavern");
        assert_eq!(ctx.tick, 100);
    }

    #[test] fn history_record_and_query() {
        let mut h = CommandHistory::new(100);
        let cmd = CommandParser::new().parse("go north").unwrap();
        let ctx = CommandContext::new(1, "lobby", 0);
        h.record(cmd, ctx, CommandResult::Success);
        assert_eq!(h.len(), 1);
        assert_eq!(h.success_rate(), 1.0);
    }

    #[test] fn history_max_size() {
        let mut h = CommandHistory::new(3);
        for i in 0..5 {
            let cmd = Command { verb: format!("cmd{}", i), args: vec![], raw: format!("cmd{}", i) };
            let ctx = CommandContext::new(1, "room", i);
            h.record(cmd, ctx, CommandResult::Success);
        }
        assert_eq!(h.len(), 3);
    }

    #[test] fn history_by_agent() {
        let mut h = CommandHistory::new(100);
        for agent in [1u64, 2] {
            let cmd = Command { verb: "go".into(), args: vec![], raw: "go".into() };
            let ctx = CommandContext::new(agent, "room", 0);
            h.record(cmd, ctx, CommandResult::Success);
        }
        assert_eq!(h.by_agent(1).len(), 1);
        assert_eq!(h.by_agent(2).len(), 1);
    }

    #[test] fn history_by_verb() {
        let mut h = CommandHistory::new(100);
        for verb in ["go", "look", "go"] {
            let cmd = Command { verb: verb.into(), args: vec![], raw: verb.into() };
            let ctx = CommandContext::new(1, "room", 0);
            h.record(cmd, ctx, CommandResult::Success);
        }
        assert_eq!(h.by_verb("go").len(), 2);
    }

    #[test] fn dispatcher_full_flow() {
        let mut d = CommandDispatcher::new(100);
        d.registry().register("go", mock_handler);
        let ctx = CommandContext::new(1, "lobby", 0);
        let result = d.dispatch("go north", &ctx);
        assert_eq!(result, Some(CommandResult::Success));
        assert_eq!(d.history().len(), 1);
    }

    #[test] fn dispatcher_unknown_command() {
        let mut d = CommandDispatcher::new(100);
        let ctx = CommandContext::new(1, "lobby", 0);
        assert_eq!(d.dispatch("fly", &ctx), Some(CommandResult::Failed));
    }

    #[test] fn dispatcher_with_alias() {
        let mut d = CommandDispatcher::new(100);
        d.registry().register("inventory", mock_handler);
        d.registry().add_alias("inv", "inventory");
        let ctx = CommandContext::new(1, "room", 0);
        assert_eq!(d.dispatch("inv", &ctx), Some(CommandResult::Success));
    }

    #[test] fn dispatcher_success_rate() {
        let mut d = CommandDispatcher::new(100);
        d.registry().register("go", mock_handler);
        d.registry().register("fail", fail_handler);
        let ctx = CommandContext::new(1, "room", 0);
        d.dispatch("go north", &ctx);
        d.dispatch("fail hard", &ctx);
        d.dispatch("go south", &ctx);
        let rate = d.history().success_rate();
        assert!((rate - 0.6667).abs() < 0.01);
    }
}

#[cfg(test)]
#[test]
fn dispatcher_empty_input() {
    let mut d = CommandDispatcher::new(100);
    let ctx = CommandContext::new(1, "room", 0);
    assert_eq!(d.dispatch("", &ctx), None);
    assert_eq!(d.history().len(), 0);
}
