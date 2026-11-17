//! The SMP's commands: their usages for every player's command tree, and a parsed command turned back into the
//! line the player typed, which the SMP's rules read.

use mod_api::{Command, CommandArgumentValue, CommandInvocation};
use std::sync::OnceLock;

/// Every usage the SMP registers.
fn usages() -> &'static [Command] {
    static USAGES: OnceLock<Vec<Command>> = OnceLock::new();
    USAGES.get_or_init(|| {
        let mut all = Vec::new();
        crate::glue::commands(&mut all);
        crate::invsee_glue::commands(&mut all);
        crate::ping_glue::commands(&mut all);
        all
    })
}

/// The SMP's commands, for the registration hook.
pub fn registered(commands: &mut Vec<Command>) {
    commands.extend(usages().iter().cloned());
}

enum Token<'a> {
    Literal(&'a str),
    /// `<name:kind>` (required) or `[name:kind]` (optional).
    Argument(&'a str, bool),
}

fn tokens(usage: &str) -> Vec<Token<'_>> {
    usage
        .split_whitespace()
        .map(|t| match t.as_bytes().first() {
            Some(b'<') | Some(b'[') => {
                let name = t
                    .trim_matches(['<', '>', '[', ']'])
                    .split(':')
                    .next()
                    .unwrap_or_default();
                Token::Argument(name, t.starts_with('<'))
            }
            _ => Token::Literal(t),
        })
        .collect()
}

fn text(
    item_id: &dyn Fn(&mod_api::ItemStack) -> String,
    value: &CommandArgumentValue,
) -> Option<String> {
    Some(match value {
        CommandArgumentValue::String(s) => s.clone(),
        CommandArgumentValue::Integer(i) => i.to_string(),
        CommandArgumentValue::Number(n) => n.to_string(),
        CommandArgumentValue::Boolean(b) => b.to_string(),
        CommandArgumentValue::Item(stack) => item_id(stack),
        _ => return None,
    })
}

/// The line a player typed, without its slash, from the usage the command matched: its words, with each
/// argument the command carries in its place.
pub fn line(
    item_id: &dyn Fn(&mod_api::ItemStack) -> String,
    command: &CommandInvocation,
) -> Option<String> {
    let literals: Vec<&str> = command.literals.iter().map(String::as_str).collect();
    'usage: for usage in usages() {
        let toks = tokens(&usage.usage);
        let words: Vec<&str> = toks
            .iter()
            .filter_map(|t| {
                if let Token::Literal(w) = t {
                    Some(*w)
                } else {
                    None
                }
            })
            .collect();
        if words != literals {
            continue;
        }
        let mut parts: Vec<String> = Vec::new();
        let mut used = 0;
        for t in &toks {
            match t {
                Token::Literal(w) => parts.push((*w).to_owned()),
                Token::Argument(name, required) => match command.argument(name) {
                    Some(v) => {
                        parts.push(text(item_id, v)?);
                        used += 1;
                    }
                    None if *required => continue 'usage,
                    None => {}
                },
            }
        }
        if used == command.arguments.len() {
            return Some(parts.join(" "));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::{CommandArgument, DimensionKey};

    fn invocation(
        literals: &[&str],
        arguments: Vec<(&str, CommandArgumentValue)>,
    ) -> CommandInvocation {
        CommandInvocation::new(
            literals.iter().map(|s| (*s).to_owned()).collect(),
            arguments
                .into_iter()
                .map(|(n, v)| CommandArgument::new(n.to_owned(), v))
                .collect(),
            DimensionKey::OVERWORLD,
            [0.0; 3],
        )
    }

    fn line_of(literals: &[&str], arguments: Vec<(&str, CommandArgumentValue)>) -> Option<String> {
        line(
            &|_| unreachable!("no item argument"),
            &invocation(literals, arguments),
        )
    }

    fn word(s: &str) -> CommandArgumentValue {
        CommandArgumentValue::String(s.to_owned())
    }

    #[test]
    fn a_parsed_command_is_the_line_the_player_typed() {
        assert_eq!(line_of(&["balance"], vec![]).as_deref(), Some("balance"));
        assert_eq!(
            line_of(&["balance"], vec![("player", word("Ann"))]).as_deref(),
            Some("balance Ann")
        );
        assert_eq!(
            line_of(
                &["pay"],
                vec![
                    ("player", word("Ann")),
                    ("amount", CommandArgumentValue::Integer(250))
                ]
            )
            .as_deref(),
            Some("pay Ann 250")
        );
        assert_eq!(
            line_of(
                &["sell", "hand"],
                vec![("count", CommandArgumentValue::Integer(5))]
            )
            .as_deref(),
            Some("sell hand 5")
        );
        assert_eq!(
            line_of(&["sell", "all"], vec![]).as_deref(),
            Some("sell all")
        );
        assert_eq!(
            line_of(&["ah", "search"], vec![("words", word("iron pickaxe"))]).as_deref(),
            Some("ah search iron pickaxe")
        );
    }

    #[test]
    fn commands_sharing_their_words_are_told_apart_by_their_arguments() {
        // `/nick <name>` (everyone) and `/nick <name> <nick>` (operators) are both `nick`.
        assert_eq!(
            line_of(&["nick"], vec![("name", word("Bob"))]).as_deref(),
            Some("nick Bob")
        );
        assert_eq!(
            line_of(
                &["nick"],
                vec![("name", word("Ann")), ("nick", word("Annie"))]
            )
            .as_deref(),
            Some("nick Ann Annie")
        );
        assert_eq!(line_of(&["vanish"], vec![]).as_deref(), Some("vanish"));
        assert_eq!(
            line_of(&["vanish", "list"], vec![]).as_deref(),
            Some("vanish list")
        );
    }

    #[test]
    fn what_the_smp_does_not_register_is_not_its_command() {
        assert_eq!(
            line_of(&["gamemode"], vec![("mode", word("creative"))]),
            None
        );
        assert_eq!(line_of(&["pay"], vec![]), None);
    }

    #[test]
    fn every_registered_usage_comes_back_as_itself() {
        for usage in usages() {
            let toks = tokens(&usage.usage);
            let literals: Vec<&str> = toks
                .iter()
                .filter_map(|t| {
                    if let Token::Literal(w) = t {
                        Some(*w)
                    } else {
                        None
                    }
                })
                .collect();
            let arguments: Vec<(&str, CommandArgumentValue)> = toks
                .iter()
                .filter_map(|t| match t {
                    Token::Argument(name, _) => Some((*name, word("1"))),
                    Token::Literal(_) => None,
                })
                .collect();
            let line = line_of(&literals, arguments);
            let want = usage
                .usage
                .split_whitespace()
                .map(|t| if t.starts_with(['<', '[']) { "1" } else { t })
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(line.as_deref(), Some(want.as_str()), "{}", usage.usage);
        }
    }
}
