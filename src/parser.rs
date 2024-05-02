use std::sync::OnceLock;

use thiserror::Error;
use url_cleaner::types::*;
use ::regex::{RegexBuilder, Regex};

use super::*;

mod domain;
mod path;
mod query;
mod regex;
mod removeparam;

pub use domain::*;
pub use path::*;
pub use query::*;
pub use regex::*;
pub use removeparam::*;

#[derive(Debug, Error)]
pub enum AdGuardError {
    #[error(transparent)]
    RegexError(#[from] ::regex::Error),
    #[error(transparent)]
    IoError(#[from] io::Error)
}

static PARSER: OnceLock<Regex> = OnceLock::new();

pub fn get_parser() -> &'static Regex {
    PARSER.get_or_init(|| RegexBuilder::new(r"^
        (?<negation>@@)?
        (?<unqualified>\|\|)?
        (?<host>[\w\-*.]+)?
        (?<path>/[^?&]*)?
        (?:[?&](?<query>.+?))?
        (?:[^a-zA-Z\d_\-.%])?
        (?:\^?\$(?:(removeparam(?:=(?<removeparam>(\\,|[^,])+)|(?<removequery>))|domain=(?<domains>[^,]+)|[^,]+),?)+)
        $")
        .multi_line(true).ignore_whitespace(true).build().unwrap())
}

pub struct AdGuardRule {
    pub negation: bool,
    pub host: Option<Condition>,
    pub path: Option<Condition>,
    pub query: Option<Condition>,
    pub removeparam: Option<Mapper>,
    pub domains: Option<Condition>
}

#[derive(Debug, Error)]
#[error("AdGuard parse error")]
pub struct AdGuardParseError;

impl FromStr for AdGuardRule {
    type Err = AdGuardParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rule = get_parser().captures(s).ok_or(AdGuardParseError)?;
        Ok(AdGuardRule {
            negation   : rule.name("negation"   ).is_some(),
            host       : rule.name("host"       ).map(|host   | domain_glob_to_condition(rule.name("unqualified").is_some(), host.as_str())),
            path       : rule.name("path"       ).map(|path   | path_glob_to_condition  (path   .as_str())),
            query      : rule.name("query"      ).map(|query  | query_to_condition      (query  .as_str()).unwrap()),
            removeparam: rule.name("removeparam").map(|query  | Mapper::try_from(removeparam::RemoveParam::from_str(query.as_str()).unwrap()).unwrap()),
            domains    : rule.name("domains"    ).map(|domains| domains_to_condition    (domains.as_str()))
        })
    }
}

impl From<AdGuardRule> for Rule {
    fn from(value: AdGuardRule) -> Self {
        let x = simplify_rule(Rule::Normal {
            condition: Condition::All(vec![
                Some(Condition::Any(vec![
                    if value.host.is_none() && value.domains.is_none() {Some(Condition::Always)} else {None},
                    value.host,
                    value.domains
                ].into_iter().flatten().collect())),
                value.path,
                value.query
            ].into_iter().flatten().collect()),
            mapper: value.removeparam.unwrap_or(Mapper::None)
        });
        println!("{x:?}");
        x
    }
}

#[derive(Debug, Error)]
pub enum ApplyNegationError {
    
}

fn apply_negation(from: Rule, to: Rule) -> Result<Rule, ApplyNegationError> {
    todo!()
}
