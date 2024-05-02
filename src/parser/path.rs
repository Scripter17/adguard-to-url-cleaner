use url_cleaner::types::*;

pub fn path_glob_to_condition(path: &str) -> Condition {
    if !path.contains('*') {return Condition::PathIs(path.into());}
    match path.split('/').skip(1).collect::<Vec<_>>()[..] {
        ["*"] => Condition::Not(Box::new(Condition::PathIs("/".into()))),
        ref x => {
            Condition::All(x.iter().enumerate().map(|(i, segment)| match segment.chars().collect::<Vec<_>>()[..] {
                ['*'] => Condition::Not(Box::new(Condition::PartIs{part: UrlPart::PathSegment(i as isize), value: None})),
                ['*', ref x @ ..     ] if !x.contains(&'*') => Condition::PartContains{part: UrlPart::PathSegment(i as isize), r#where: StringLocation::End  , value:      x.iter().collect::<String>().into()} ,
                [     ref x @ .., '*'] if !x.contains(&'*') => Condition::PartContains{part: UrlPart::PathSegment(i as isize), r#where: StringLocation::Start, value:      x.iter().collect::<String>().into()} ,
                      ref x            if !x.contains(&'*') => Condition::PartIs      {part: UrlPart::PathSegment(i as isize),                                 value: Some(x.iter().collect::<String>().into())},
                _ => todo!()
            }).collect::<Vec<_>>())
        }
    }
}

