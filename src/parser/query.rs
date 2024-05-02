use thiserror::Error;
use url_cleaner::types::*;

use super::*;

#[derive(Debug, Error)]
pub enum QueryToConditionError {
    #[error(transparent)]
    RegexMakingError(#[from] Box<regex_syntax::Error>)
}

pub fn query_to_condition(query: &str) -> Result<Condition, QueryToConditionError> {
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

