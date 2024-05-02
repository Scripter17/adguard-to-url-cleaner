use url_cleaner::types::*;

pub fn domain_glob_to_condition(explicitly_unqualified: bool, domain: &str) -> Condition {
    match (explicitly_unqualified, &domain.split('.').collect::<Vec<_>>()[..]) {
        (_    , ["*", ref segments @ .., "*"]) | (true , [ref segments @ .., "*"]) if !segments.contains(&"*") => Condition::UnqualifiedAnyTld(domain.strip_suffix(".*").unwrap().to_string()),
        (false, [     ref segments @ .., "*"])                                     if !segments.contains(&"*") => Condition::QualifiedAnyTld  (domain.strip_suffix(".*").unwrap().to_string()),
        (true ,           segments           )                                     if !segments.contains(&"*") => Condition::UnqualifiedDomain(domain.to_string()),
        (false,           segments           )                                     if !segments.contains(&"*") => Condition::QualifiedDomain  (domain.to_string()),
        _ => todo!()
    }
}

pub fn domains_to_condition(domains: &str) -> Condition {
    Condition::Any(domains.split('|').map(|domain| domain_glob_to_condition(true, domain)).collect())
}
