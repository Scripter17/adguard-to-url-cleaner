use url_cleaner::types::*;

pub fn simplify_rules(rules: Rules) -> Rules {
    let mut ret = Vec::new();

    for rule in rules.0.into_iter().map(simplify_rule) {
        if let Some(last_rule) = ret.last_mut() {
            if let (Rule::Normal{condition: ref last_condition, mapper: ref mut last_mapper}, Rule::Normal{condition, mapper}) = (last_rule, &rule) {
                 if last_condition == condition {
                    if let (Mapper::RemoveQueryParams(ref mut last_params), Mapper::RemoveQueryParams(params)) = (last_mapper, mapper) {
                        last_params.extend(params.clone());
                        continue;
                    }
                }
            }
        }
        if !matches!(rule, Rule::Normal{condition: Condition::Never, ..}) {
            ret.push(rule);
        }
    }
    Rules(ret)
}

pub fn simplify_rule(rule: Rule) -> Rule {
    match rule {
        Rule::Normal{condition, mapper} => Rule::Normal {condition: simplify_condition(condition), mapper: simplify_mapper(mapper)},
        _ => rule
    }
}

pub fn simplify_condition(condition: Condition) -> Condition {
    match condition {
        Condition::Any(subconditions) => {
            let mut subconditions = subconditions.into_iter().filter_map(|x| {let ret = simplify_condition(x); if ret != Condition::Never {Some(ret)} else {None}}).collect::<Vec<_>>();
            match subconditions.len() {
                0 => Condition::Never,
                1 => simplify_condition(subconditions.remove(0)),
                _ => {
                    let mut ret = Vec::new();
                    for subcondition in subconditions {
                        match simplify_condition(subcondition) {
                            Condition::Always => return Condition::Always,
                            Condition::Any(subsubconditions) => {ret.extend(subsubconditions)},
                            subsubcondition => {ret.push(subsubcondition)}
                        }
                    }
                    Condition::Any(ret)
                }
            }
        },
        Condition::All(subconditions) => {
            let mut subconditions = subconditions.into_iter().filter_map(|x| {let ret = simplify_condition(x); if ret != Condition::Always {Some(ret)} else {None}}).collect::<Vec<_>>();
            match subconditions.len() {
                0 => Condition::Always,
                1 => simplify_condition(subconditions.remove(0)),
                _ => {
                    let mut ret = Vec::new();
                    for subcondition in subconditions {
                        match simplify_condition(subcondition) {
                            Condition::Never => return Condition::Never,
                            Condition::All(subsubconditions) => {ret.extend(subsubconditions)},
                            subsubcondition => {ret.push(subsubcondition)}
                        }
                    }
                    Condition::All(ret)
                }
            }
        },
        _ => condition
    }
}

pub fn simplify_mapper(mapper: Mapper) -> Mapper {
    mapper
}

pub fn condition_is_partial_non_strict_subset(left: &Condition, right: &Condition) -> bool {
    // LEFT IS STRICTER.
    if left==right {return true;}

    match (left, right) {
        (Condition::Never, _) => true,
        (_, Condition::Always) => true,
        (Condition::All(ls), Condition::All(rs)) => ls.len() >= rs.len() && rs.iter().all(|r| ls.iter().any(|l| condition_is_partial_non_strict_subset(l, r))),
        (Condition::Any(ls), Condition::Any(rs)) => ls.len() <= rs.len() && ls.iter().all(|l| rs.iter().any(|r| condition_is_partial_non_strict_subset(l, r))),
        (Condition::QualifiedDomain  (l), Condition::UnqualifiedDomain(r)) => l.split('.').collect::<Vec<_>>().ends_with(&r.split('.').collect::<Vec<_>>()),
        (Condition::QualifiedDomain  (l), Condition::QualifiedAnyTld  (r)) => l.strip_suffix(psl::suffix_str(l).unwrap()).unwrap().strip_suffix('.').unwrap()==r,
        (Condition::QualifiedDomain  (l), Condition::UnqualifiedAnyTld(r)) => l.strip_suffix(psl::suffix_str(l).unwrap()).unwrap().strip_suffix('.').unwrap().split('.').collect::<Vec<_>>().ends_with(&r.split('.').collect::<Vec<_>>()),
        (Condition::UnqualifiedDomain(l), Condition::UnqualifiedAnyTld(r)) => l.strip_suffix(psl::suffix_str(l).unwrap()).unwrap().strip_suffix('.').unwrap().split('.').collect::<Vec<_>>().ends_with(&r.split('.').collect::<Vec<_>>()),
        _ => false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cipnss() {
        assert!(condition_is_partial_non_strict_subset(&Condition::All(vec![Condition::Never]), &Condition::All(vec![Condition::QualifiedDomain("abc.com".to_string())])));
        assert!(condition_is_partial_non_strict_subset(&Condition::All(vec![Condition::QualifiedDomain("abc.xyz.com".to_string())]), &Condition::All(vec![Condition::UnqualifiedDomain("xyz.com".to_string())])));
        assert!(condition_is_partial_non_strict_subset(&Condition::All(vec![Condition::QualifiedDomain("abc.xyz.com".to_string())]), &Condition::All(vec![Condition::QualifiedAnyTld("abc.xyz".to_string())])));
        assert!(condition_is_partial_non_strict_subset(&Condition::All(vec![Condition::QualifiedDomain("abc.xyz.com".to_string())]), &Condition::All(vec![Condition::UnqualifiedAnyTld("abc.xyz".to_string())])));
        assert!(condition_is_partial_non_strict_subset(&Condition::All(vec![Condition::UnqualifiedDomain("abc.xyz.com".to_string())]), &Condition::All(vec![Condition::UnqualifiedAnyTld("xyz".to_string())])));
        assert!(condition_is_partial_non_strict_subset(&Condition::All(vec![Condition::UnqualifiedDomain("xyz.com".to_string())]), &Condition::All(vec![Condition::UnqualifiedAnyTld("xyz".to_string())])));
        // assert!(condition_is_partial_non_strict_subset(Condition::All(vec![]), Condition::All(vec![])));
        // assert!(condition_is_partial_non_strict_subset(Condition::All(vec![]), Condition::All(vec![])));
        // assert!(condition_is_partial_non_strict_subset(Condition::All(vec![]), Condition::All(vec![])));
    }
}
