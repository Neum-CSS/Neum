use crate::error::{ErrorType, NeumError};
use crate::lexer::Token;
use core::slice::Iter;
use regex::Regex;
use std::collections::HashMap;
use std::ops::Range;

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Name {
    pub regex: Regex,
    pub variables: Vec<String>,
}

pub fn parse<S: AsRef<str>>(
    tokens: Vec<(Token, Range<usize>)>,
    file: Option<S>,
    content: S,
) -> Result<Vec<(Name, Vec<Token>)>, NeumError> {
    let file = file.map(|x| x.as_ref().to_string());
    let mut list = Vec::new();
    let mut token = tokens.iter();
    while let Some(next) = token.next() {
        match next.0 {
            Token::String(_) => {
                let mut name = vec![next.clone()];
                let mut last = next;
                for i in token.by_ref() {
                    last = i;
                    if i.0 != Token::ConvertTo {
                        name.push(i.clone());
                    } else {
                        break;
                    }
                }

                let mut variables: Vec<String> = Vec::new();
                let mut regex = "^".to_string();
                let mut name_iter = name.iter();
                while let Some(i) = name_iter.next() {
                    let value = match &i.0 {
                        Token::ReplacementStart => {
                            let next = name_iter
                                .next()
                                .ok_or_else(|| {
                                    NeumError::new(
                                        ErrorType::UnexpectedEndOfFile,
                                        file.clone(),
                                        content.as_ref().to_string(),
                                        i.1.end..i.1.end + 1,
                                    )
                                })?
                                .clone();
                            if let Token::String(x) = &next.0 {
                                if variables.contains(x) {
                                    return Err(NeumError::new(
                                        ErrorType::VariableMultiDefine,
                                        file,
                                        content.as_ref().to_string(),
                                        next.1,
                                    ));
                                }
                                variables.push(x.to_string());
                                let next_name = name_iter.next().ok_or_else(|| {
                                    NeumError::new(
                                        ErrorType::UnexpectedToken,
                                        file.clone(),
                                        content.as_ref().to_string(),
                                        next.clone().1,
                                    )
                                })?;
                                if next_name.0 != Token::ReplacementEnd {
                                    return Err(NeumError::new(
                                        ErrorType::UnexpectedToken,
                                        file.clone(),
                                        content.as_ref().to_string(),
                                        next.1,
                                    ));
                                }
                            } else if Token::ReplacementEnd == next.0 {
                                if variables.contains(&"".to_string()) {
                                    return Err(NeumError::new(
                                        ErrorType::VariableMultiDefine,
                                        file,
                                        content.as_ref().to_string(),
                                        next.1,
                                    ));
                                }
                                variables.push("".to_string())
                            } else {
                                return Err(NeumError::new(
                                    ErrorType::UnexpectedToken,
                                    file,
                                    content.as_ref().to_string(),
                                    next.1,
                                ));
                            }

                            Ok("(.*)".to_string())
                        }
                        Token::Add => Ok("+".to_string()),
                        Token::Subtract => Ok(r"\-".to_string()),
                        Token::Times => Ok(r"\*".to_string()),
                        Token::Divide => Ok("/".to_string()),
                        Token::Number(x) => Ok(regex::escape(x.to_string().as_str())),
                        Token::String(x) => Ok(regex::escape(x)),
                        Token::Space => Ok("".to_string()),
                        _ => Err(NeumError::new(
                            ErrorType::UnexpectedToken,
                            file.clone(),
                            content.as_ref().to_string(),
                            i.clone().1,
                        )),
                    };
                    match value {
                        Ok(x) => regex.push_str(x.as_str()),
                        Err(x) => return Err(x),
                    }
                }

                regex.push('$');

                let mut first = &token
                    .next()
                    .ok_or_else(|| {
                        NeumError::new(
                            ErrorType::UnexpectedEndOfFile,
                            file.clone(),
                            content.as_ref().to_string(),
                            last.1.end..last.1.end + 1,
                        )
                    })?
                    .0;
                if first == &Token::Space {
                    first = &token
                        .next()
                        .ok_or_else(|| {
                            NeumError::new(
                                ErrorType::UnexpectedEndOfFile,
                                file.clone(),
                                content.as_ref().to_string(),
                                last.1.end..last.1.end + 1,
                            )
                        })?
                        .0;
                }
                let mut convert_to = Vec::new();
                let go_to = match first {
                    Token::MultiEqualStart => Token::MultiEqualEnd,
                    _ => {
                        convert_to.push(first.clone());
                        Token::NewLine
                    }
                };
                let mut broke = false;
                for i in token.by_ref() {
                    last = i;
                    if i.0 != go_to {
                        if !matches!(
                            i.0,
                            Token::ReplacementStart
                                | Token::ReplacementEnd
                                | Token::Add
                                | Token::Subtract
                                | Token::Times
                                | Token::Divide
                                | Token::Number(_)
                                | Token::String(_)
                                | Token::SemiColon
                                | Token::NewLine
                                | Token::FullReplacementStart
                                | Token::FullReplacementEnd
                                | Token::Space
                        ) {
                            return Err(NeumError::new(
                                ErrorType::UnexpectedToken,
                                file,
                                content.as_ref().to_string(),
                                i.clone().1,
                            ));
                        }
                        convert_to.push(i.0.clone());
                    } else {
                        broke = true;
                        break;
                    }
                }
                if !broke {
                    return Err(NeumError::new(
                        ErrorType::UnexpectedEndOfFile,
                        file,
                        content.as_ref().to_string(),
                        last.1.end..last.1.end + 1,
                    ));
                }
                list.push((
                    Name {
                        regex: Regex::new(&regex)
                            .expect("Internal error, could not make regex from input"),
                        variables,
                    },
                    convert_to,
                ));
            }
            _ => {
                return Err(NeumError::new(
                    ErrorType::UnexpectedToken,
                    file,
                    content.as_ref().to_string(),
                    next.clone().1,
                ));
            }
        }
    }
    Ok(list)
}

pub fn converts<S: AsRef<str> + std::fmt::Display>(
    parsed: Vec<(Name, Vec<Token>)>,
    input: S,
) -> Option<String> {
    for i in &parsed {
        if let Some(caps) = i.0.regex.captures(input.as_ref()) {
            let mut caps_iter = caps.iter();
            caps_iter.next();
            let mut variables = HashMap::new();
            for x in i.0.variables.clone() {
                variables.insert(
                    x,
                    caps_iter
                        .next()
                        .unwrap_or_else(|| {
                            panic!("Internal Error\ninput: {input}\nregex: {:?}", i.0.regex)
                        })
                        .unwrap_or_else(|| {
                            panic!("Internal Error\ninput: {input}\nregex: {}", i.0.regex)
                        })
                        .as_str()
                        .to_string(),
                );
            }
            let mut returns = String::new();
            let mut returns_iter = i.1.iter();
            while let Some(x) = returns_iter.next() {
                returns.push_str(
                    match x {
                        Token::FullReplacementStart => {
                            let mut search = String::new();
                            while let Some(x) = returns_iter.next() {
                                if x == &Token::FullReplacementEnd {
                                    break;
                                }
                                search.push_str(&match x {
                                    Token::ReplacementStart => replacement(
                                        &mut returns_iter,
                                        variables.clone(),
                                        &i.clone(),
                                    ),
                                    Token::Add => "+".to_string(),
                                    Token::Subtract => r"\-".to_string(),
                                    Token::Times => r"\*".to_string(),
                                    Token::Divide => "/".to_string(),
                                    Token::Number(x) => x.to_string(),
                                    Token::String(x) => x.clone(),
                                    Token::SemiColon => ";".to_string(),
                                    Token::NewLine => ";".to_string(),
                                    _ => "".to_string(),
                                });
                            }
                            let returns = converts(parsed.clone(), search)?;
                            let mut chars = returns.chars();
                            chars.next_back();
                            chars.as_str().to_string()
                        }
                        Token::ReplacementStart => {
                            replacement(&mut returns_iter, variables.clone(), i)
                        }
                        Token::Add => "+".to_string(),
                        Token::Subtract => r"\-".to_string(),
                        Token::Times => r"\*".to_string(),
                        Token::Divide => "/".to_string(),
                        Token::Number(x) => x.to_string(),
                        Token::String(x) => x.clone(),
                        Token::SemiColon => ";".to_string(),
                        Token::NewLine => ";".to_string(),
                        Token::Space => " ".to_string(),
                        _ => "".to_string(),
                    }
                    .as_str(),
                )
            }
            if !returns.ends_with(';') {
                returns.push(';');
            }
            return Some(
                returns
                    .trim()
                    .to_string()
                    .replace("; ", ";")
                    .replace(": ", ":"),
            );
        }
    }
    None
}

fn replacement(
    returns_iter: &mut Iter<Token>,
    variables: HashMap<String, String>,
    i: &(Name, Vec<Token>),
) -> String {
    let mut next = returns_iter
        .next()
        .expect("Should never happen but failed to get value");
    if next == &Token::Space {
        next = returns_iter
            .next()
            .expect("Should never happen but failed to get value");
    }
    if next == &Token::ReplacementEnd {
        (*variables
            .get("")
            .unwrap_or_else(|| panic!("Internal Error\nCould not find variable \"\" in {:?}", i.1)))
        .clone()
    } else {
        let mut next_value = false;
        let value = match next {
            Token::String(w) => (*variables.get(w).unwrap_or_else(|| {
                panic!(
                    "Internal Error\nCould not find variable \"{}\" in {:?}",
                    w, i.1
                )
            }))
            .to_string(),
            Token::Number(n) => n.to_string(),
            Token::Add => {
                next_value = true;
                (*variables.get("").unwrap_or_else(|| {
                    panic!("Internal Error\nCould not find variable \"\" in {:?}", i.1)
                }))
                .to_string()
            }
            Token::Subtract => {
                next_value = true;
                (*variables.get("").unwrap_or_else(|| {
                    panic!("Internal Error\nCould not find variable \"\" in {:?}", i.1)
                }))
                .to_string()
            }
            Token::Times => {
                next_value = true;
                (*variables.get("").unwrap_or_else(|| {
                    panic!("Internal Error\nCould not find variable \"\" in {:?}", i.1)
                }))
                .to_string()
            }
            Token::Divide => {
                next_value = true;
                (*variables.get("").unwrap_or_else(|| {
                    panic!("Internal Error\nCould not find variable \"\" in {:?}", i.1)
                }))
                .to_string()
            }
            _ => panic!("Internal Error\nDont know what {:?} is in {:?}", next, i.1),
        };
        if returns_iter.len() > 0 {
            let mut int_value = value.parse::<f64>().unwrap_or_else(|_| {
                panic!(
                    "Internal Error\nCant do multipul things to a string, \"{}\", in {:?}",
                    value, i.1
                )
            });
            if next_value {
                let next_value = match returns_iter
                    .next()
                    .expect("Internal Error\nCould nothing after a \"+\" \"-\" \"*\" \"/\"")
                {
                    Token::String(w) => variables
                        .get(w)
                        .unwrap_or_else(|| {
                            panic!(
                                "Internal Error\nCould not find variable \"{}\" in {:?}",
                                w, i.1
                            )
                        })
                        .parse::<f64>()
                        .unwrap_or_else(|_| {
                            panic!(
                                "Internal Error\nCould not convert variable \"{}\" in {:?} to f64",
                                w, i.1
                            )
                        }),
                    Token::Number(w) => *w,
                    _ => panic!("Internal Error\nCould not find out what char is requested for"),
                };
                match next {
                    Token::Add => int_value += next_value,
                    Token::Subtract => int_value -= next_value,
                    Token::Times => int_value *= next_value,
                    Token::Divide => int_value /= next_value,
                    _ => panic!("Internal Error\nUsed a token not able to use in replacement"),
                }
            }
            while let Some(mut y) = returns_iter.next() {
                if y == &Token::Space {
                    y = returns_iter
                        .next()
                        .expect("Inetrnal Error\nCould not find end of Replacement");
                }
                if y == &Token::ReplacementEnd {
                    break;
                }
                let mut next_value = returns_iter
                    .next()
                    .expect("Internal Error\nCould nothing after a \"+\" \"-\" \"*\" \"/\"");
                if next_value == &Token::Space {
                    next_value = returns_iter
                        .next()
                        .expect("Inetrnal Error\nCould not find end of Replacement");
                }
                let next = match next_value {
                    Token::String(w) => variables
                        .get(w)
                        .unwrap_or_else(|| {
                            panic!(
                                "Internal Error\nCould not find variable \"{}\" in {:?}",
                                w, i.1
                            )
                        })
                        .parse::<f64>()
                        .unwrap_or_else(|_| {
                            panic!(
                                "Internal Error\nCould not convert variable \"{}\" in {:?} to f64",
                                w, i.1
                            )
                        }),
                    Token::Number(w) => *w,
                    _ => panic!(
                        "Internal Error\nCould not find out what char is requested for {y:?}"
                    ),
                };
                match y {
                    Token::Add => int_value += next,
                    Token::Subtract => int_value -= next,
                    Token::Times => int_value *= next,
                    Token::Divide => int_value /= next,
                    _ => panic!("Internal Error\nUsed a token not able to use in replacement"),
                }
            }
            int_value.to_string()
        } else {
            value
        }
    }
    .trim()
    .to_string()
}
