use thiserror::Error;
use url_cleaner::types::*;

use super::*;

#[derive(Debug, Clone)]
pub enum RemoveParam {
    RegexParts(RegexParts),
    String(String)
}

#[derive(Debug, Error)]
pub enum ParseRemoveParamError {
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
            RemoveParam::RegexParts(regex_parts) => Self::RemoveQueryParamsMatching(StringMatcher::Regex(regex_parts.into())),
            RemoveParam::String(string) => Self::RemoveQueryParams([string].into_iter().collect())
        })
    }
}

