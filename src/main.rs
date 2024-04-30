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
    RegexSyntaxError(#[from] Box<regex_syntax::Error>)
}

impl FromStr for RemoveParam {
    type Err = ParseRemoveParamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s.starts_with('/') {
            Self::RegexParts(RegexParts::new_with_config(
                s.split_once('/').unwrap().1.rsplit_once('/').unwrap().0,
                {let mut config = RegexConfig::default(); config.add_flags(s.rsplit_once('/').unwrap().1); config}
            )?)
        } else {
            Self::String(s.to_owned())
        })
    }
}

fn domain_glob_to_condition(unqualified: bool, domain: &str) -> Condition {
    match (unqualified, &domain.split('.').collect::<Vec<_>>()[..]) {
        (_    , ["*", ref segments @ .., "*"]) | (true, [ref segments @ .., "*"]) if !segments.contains(&"*") => Condition::UnqualifiedAnyTld(domain.to_string()),
        (false, [     ref segments @ .., "*"]) | (true, [ref segments @ ..     ]) if !segments.contains(&"*") => Condition::QualifiedAnyTld  (domain.to_string()),
        (true ,       ref segments           )                                    if !segments.contains(&"*") => Condition::UnqualifiedDomain(domain.to_string()),
        (false,       ref segments           )                                    if !segments.contains(&"*") => Condition::QualifiedDomain  (domain.to_string()),
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
            let host        = rule.name("host"       ).map(|host   | host   .as_str());
            let path        = rule.name("path"       ).map(|path   | path   .as_str());
            let query       = rule.name("query"      ).map(|query  | query  .as_str());
            let removeparam = rule.name("removeparam").map(|query  | query  .as_str());
            let domains     = rule.name("domains"    ).map(|domains| domains.as_str());
            println!("{line}");
            println!("{negation:?} {unqualified:?} {host:?} {path:?} {query:?} {removeparam:?} {domains:?}");
            if let Some(host) = host {println!("-- {:?}", domain_glob_to_condition(unqualified, host));}
            if let Some(path) = path {println!("-- {:?}", path_glob_to_condition(path));}
            if let Some(removeparam) = removeparam {println!("-- {:?}", RemoveParam::from_str(removeparam));}
            println!();
        } else if !line.starts_with('!') {
            eprintln!("Non-comment line not parsed: {line}");
        }
    }

    Ok(())
}
