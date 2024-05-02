use thiserror::Error;

#[derive(Debug, Error)]
pub enum UDERPError {
    #[error("UnknownEscape")]
    UnknownEscape
}

pub fn un_double_escape_regex_pattern(pattern: &str) -> Result<String, UDERPError> {
    let mut ret = String::new();
    let mut escape = false;

    for c in pattern.chars() {
        if let Some(to_push) = match (escape, c) {
            (false, '\\') => {escape = true ; None      },
            (true , '\\') => {escape = false; Some('\\')},
            (true , '/' ) => {escape = false; Some('/' )},
            (true , ',' ) => {escape = false; Some(',' )},
            _ => Some(c)
        } {
            ret.push(to_push);
        }
    }
    Ok(ret)
}

