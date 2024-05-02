use std::str::FromStr;
use std::fs::File;
use std::io::{self, prelude::*, BufReader};

use url_cleaner::{
    types::*,
    glue::*
};

mod eqmap;
mod parser;
mod rules;

use eqmap::*;
use rules::*;

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


fn main() -> Result<(), parser::AdGuardError> {
    // https://adguard.com/kb/general/ad-filtering/create-own-filters
    let mut rules=Vec::new();
    let mut whitelist_rules=Vec::new();

    for line in BufReader::new(File::open("rules.txt")?).lines() {
        let line = line?;
        if let Ok(rule) = parser::AdGuardRule::from_str(&line) {
            if rule.negation {
                whitelist_rules.push(rule.into());
            } else {
                rules.push(rule.into());
            }
        } else if !line.starts_with('!') {
            eprintln!("Non-comment line not parsed: {line}");
        }
    }

    for whitelist_rule in whitelist_rules.iter() {
        for rule in rules.iter_mut() {
            match (whitelist_rule, rule) {
                (Rule::Normal{condition: wc, mapper: wm}, Rule::Normal{condition: ref mut rc, mapper: rm}) => {
                    if wm==rm && condition_is_partial_non_strict_subset(&wc, &rc) {
                        println!("{wc:?} {wm:?} {rc:?} {rm:?}");
                        *rc = Condition::All(vec![Condition::Not(Box::new(wc.clone())), rc.clone()])
                    }
                },
                _ => panic!()
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&simplify_rules(Rules(rules))).unwrap());

    Ok(())
}
