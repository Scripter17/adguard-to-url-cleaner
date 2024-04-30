use std::str::FromStr;
use std::fs::File;
use std::io::{self, prelude::*, BufReader};

use thiserror::Error;
use url_cleaner::{
    types::*,
    glue::*
};

fn split_on_pipe_but_not_in_regex(x: &str) -> Vec<String> {
    let mut escaped=false;
    let mut split=true;
    let mut acc=String::new();
    let mut ret=Vec::new();
    for c in x.chars() {
        if c=='\\' {escaped = !escaped;}
        if c=='/' && !escaped {split = !split;}
        if c=='|' && split {
            ret.push(acc.replace("\\,", ","));
            acc=String::new();
        } else {
            acc.push(c);
        }
    }
    if !acc.is_empty() {ret.push(acc.replace("\\,", ","));}
    ret
}

#[derive(Debug, Clone)]
enum RemoveParam {
    RegexParts(RegexParts),
    String(String)
}

#[derive(Debug, Error)]
enum ParseRemoveParamError {
    #[error(transparent)]
    RegexSyntaxError(#[from] Box<regex_syntax::Error>),
    #[error(transparent)]
    USERPError(#[from] UDERPError)
}

impl FromStr for RemoveParam {
    type Err = ParseRemoveParamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s.starts_with('/') {
            Self::RegexParts(RegexParts::new_with_config(
                &un_double_escape_regex_pattern(s.split_once('/').unwrap().1.rsplit_once('/').unwrap().0)?,
                // s.split_once('/').unwrap().1.rsplit_once('/').unwrap().0,
                {let mut config = RegexConfig::default(); config.add_flags(s.rsplit_once('/').unwrap().1); config}
            )?)
        } else {
            Self::String(s.to_owned())
        })
    }
}

impl TryFrom<RemoveParam> for Mapper {
    type Error = <RegexParts as TryInto<RegexWrapper>>::Error;
    
    fn try_from(value: RemoveParam) -> Result<Self, <Self as TryFrom<RemoveParam>>::Error> {
        Ok(match value {
            RemoveParam::RegexParts(regex_parts) => Self::RemoveQueryParamsMatching(StringMatcher::Regex(regex_parts.try_into()?)),
            RemoveParam::String(string) => Self::RemoveQueryParams([string].into_iter().collect())
        })
    }
}

#[derive(Debug, Error)]
enum UDERPError {
    #[error("UnknownEscape")]
    UnknownEscape
}

fn un_double_escape_regex_pattern(pattern: &str) -> Result<String, UDERPError> {
    let mut ret = String::new();
    let mut escape = false;

    for c in pattern.chars() {
        if let Some(to_push) = match (escape, c) {
            (false, '\\') => {escape = true ; None      },
            (true , '\\') => {escape = false; Some('\\')},
            (true , '/' ) => {escape = false; Some('/' )},
            (true , ',' ) => {escape = false; Some(',' )},
            (true , _   ) => {None},// Err(UDERPError::UnknownEscape)?,
            (false, _   ) => Some(c)
        } {
            ret.push(to_push);
        }
    }
    Ok(ret)
}

fn domain_glob_to_condition(explicitly_unqualified: bool, domain: &str) -> Condition {
    match (explicitly_unqualified, &domain.split('.').collect::<Vec<_>>()[..]) {
        (_    , ["*", ref segments @ .., "*"]) | (true , [ref segments @ .., "*"]) if !segments.contains(&"*") => Condition::UnqualifiedAnyTld(domain.strip_suffix(".*").unwrap().to_string()),
        (false, [     ref segments @ .., "*"])                                     if !segments.contains(&"*") => Condition::QualifiedAnyTld  (domain.strip_suffix(".*").unwrap().to_string()),
        (true ,       ref segments           )                                     if !segments.contains(&"*") => Condition::UnqualifiedDomain(domain.to_string()),
        (false,       ref segments           )                                     if !segments.contains(&"*") => Condition::QualifiedDomain  (domain.to_string()),
        _ => todo!()
    }
}

fn path_glob_to_condition(path: &str) -> Condition {
    if !path.contains('*') {return Condition::PathIs(path.into());}
    match path.split('/').skip(1).collect::<Vec<_>>()[..] {
        ["*"] => Condition::Not(Box::new(Condition::PathIs("/".into()))),
        ref x => {
            Condition::All(x.into_iter().enumerate().map(|(i, segment)| match segment.chars().collect::<Vec<_>>()[..] {
                ['*'] => Condition::Not(Box::new(Condition::PartIs{part: UrlPart::PathSegment(i as isize), value: None})),
                ['*', ref x @ ..     ] if !x.contains(&'*') => Condition::PartContains{part: UrlPart::PathSegment(i as isize), r#where: StringLocation::End  , value:      x.into_iter().collect::<String>().into()} ,
                [     ref x @ .., '*'] if !x.contains(&'*') => Condition::PartContains{part: UrlPart::PathSegment(i as isize), r#where: StringLocation::Start, value:      x.into_iter().collect::<String>().into()} ,
                      ref x            if !x.contains(&'*') => Condition::PartIs      {part: UrlPart::PathSegment(i as isize),                                 value: Some(x.into_iter().collect::<String>().into())},
                _ => todo!()
            }).collect::<Vec<_>>())
        }
    }
}

#[derive(Debug, Error)]
enum QueryToConditionError {
    #[error(transparent)]
    RegexMakingError(#[from] Box<regex_syntax::Error>)
}

fn query_to_condition(query: &str) -> Result<Condition, QueryToConditionError> {
    Ok(Condition::All(query.split('&').map(|param| Ok(match param.split_once('=') {
        None => Condition::Not(Box::new(Condition::PartIs{part: UrlPart::QueryParam(param.to_string()), value: None})),
        Some((k, v)) => match (k, v) {
            (k,  v ) if !k.contains('*') && !v.contains('*') => Condition::PartIs{part: UrlPart::QueryParam(k.to_string()), value: Some(v.into())},
            (k, "*") if !k.contains('*')                     => Condition::Not(Box::new(Condition::PartIs{part: UrlPart::QueryParam(k.to_string()), value: None})),
            (k,  v ) if !k.contains('*')                     => Condition::PartMatches{part: UrlPart::QueryParam(k.to_string()), matcher: StringMatcher::Regex(RegexWrapper::from_str(&v.replace('*', ".*"))?)},
            _ => todo!()
        }
    })).collect::<Result<_, QueryToConditionError>>()?))
}

fn domains_to_condition(domains: &str) -> Condition {
    Condition::Any(domains.split('|').map(|domain| domain_glob_to_condition(true, domain)).collect())
}

#[derive(Debug, Error)]
enum AdGuardError {
    #[error(transparent)]
    RegexError(#[from] regex::Error),
    #[error(transparent)]
    IoError(#[from] io::Error)
}

fn main() -> Result<(), AdGuardError> {
    // https://adguard.com/kb/general/ad-filtering/create-own-filters
    let rule_parser = regex::RegexBuilder::new(r"^
        (?<negation>@@)?
        (?<unqualified>\|\|)?
        (?<host>[\w\-*.]+)?
        (?<path>/[^?&]*)?
        (?:[?&](?<query>.+?))?
        (?:[^a-zA-Z\d_\-.%])?
        (?:\^?\$(?:(removeparam(?:=(?<removeparam>(\\,|[^,])+)|(?<removequery>))|domain=(?<domains>[^,]+)|[^,]+),?)+)
        $")
        .multi_line(true).ignore_whitespace(true).build()?;

    let mut rules=Vec::<Rule>::new();

    for line in BufReader::new(File::open("rules.txt")?).lines() {
        let line = line?;
        if let Some(rule) = rule_parser.captures(&line) {
            let negation    = rule.name("negation"   ).is_some();
            let unqualified = rule.name("unqualified").is_some();
            let host        = rule.name("host"       ).map(|host   | domain_glob_to_condition(unqualified, host.as_str()));
            let path        = rule.name("path"       ).map(|path   | path_glob_to_condition  (path   .as_str()));
            let query       = rule.name("query"      ).map(|query  | query_to_condition      (query  .as_str()).unwrap());
            let removeparam = rule.name("removeparam").map(|query  | Mapper::try_from(RemoveParam::from_str(query.as_str()).unwrap()).unwrap());
            let domains     = rule.name("domains"    ).map(|domains| domains_to_condition    (domains.as_str()));
            if negation {
                println!("-- {line}");
                println!("TODO: Negated rules.");
            } else if let Some(removeparam) = removeparam {
                let rule = simplify_rule(Rule::Normal {
                    condition: Condition::All(vec![Some(Condition::Any(vec![host, domains].into_iter().filter_map(|x| x).collect())), path, query].into_iter().filter_map(|x| x).collect()),
                    mapper: removeparam
                });
                println!("-- {line}");
                println!("{}", serde_json::to_string_pretty(&rule).unwrap());
            }
        } else if !line.starts_with('!') {
            eprintln!("Non-comment line not parsed: {line}");
        }
    }

    Ok(())
}

fn simplify_rule(rule: Rule) -> Rule {
    match rule {
        Rule::Normal{condition, mapper} => Rule::Normal {condition: simplify_condition(condition), mapper: simplify_mapper(mapper)},
        _ => rule
    }
}

fn simplify_condition(condition: Condition) -> Condition {
    match condition {
        Condition::Any(subconditions) if subconditions.len() == 0 => Condition::Never,
        Condition::Any(subconditions) if subconditions.len() == 1 => simplify_condition(subconditions.get(0).unwrap().clone()), // `subconditions[0]` doesn't work (probably fixed by Polonious)
        Condition::Any(subconditions) => {
            let mut ret = Vec::new();
            for subcondition in subconditions {
                match simplify_condition(subcondition) {
                    Condition::Any(subsubconditions) => {ret.extend(subsubconditions)},
                    subcondition => {ret.push(subcondition)}
                }
            }
            Condition::Any(ret)
        },
        Condition::All(subconditions) if subconditions.len() == 0 => Condition::Always,
        Condition::All(subconditions) if subconditions.len() == 1 => simplify_condition(subconditions.get(0).unwrap().clone()), // `subconditions[0]` doesn't work (probably fixed by Polonious)
        Condition::All(subconditions) => {
            let mut ret = Vec::new();
            for subcondition in subconditions {
                match simplify_condition(subcondition) {
                    Condition::All(subsubconditions) => {ret.extend(subsubconditions)},
                    subcondition => {ret.push(subcondition)}
                }
            }
            Condition::All(ret)
        },
        _ => condition
    }
}

fn simplify_mapper(mapper: Mapper) -> Mapper {
    mapper
}
