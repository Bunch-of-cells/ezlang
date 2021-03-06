use std::{collections::HashSet, fs, rc::Rc};

use crate::{
    lexer,
    utils::{Error, ErrorType, Token, TokenType},
};

pub fn preprocess(mut tokens: Vec<Token>) -> Result<Vec<Token>, Error> {
    let mut declared = HashSet::new();
    let mut i = 0;
    let mut ifs = Vec::new();
    while i < tokens.len() {
        if let TokenType::PreprocessorStatement(ref stmt) = tokens[i].token_type {
            match stmt.as_ref() {
                "use" => match tokens.get(i + 1).cloned() {
                    None => {
                        return Err(Error::new(
                            ErrorType::SyntaxError,
                            tokens[i].position.clone(),
                            "Expected a filename after `use`".to_owned(),
                        ))
                    }
                    Some(t) => match t.token_type {
                        TokenType::String(file) => match fs::read_to_string(&file) {
                            Ok(contents) => {
                                let mut new_tokens = lexer::lex(&contents, Rc::new(file))?;
                                new_tokens.pop().unwrap();
                                tokens.splice(i..=i + 1, new_tokens);
                            }
                            Err(e) => {
                                return Err(Error::new(
                                    ErrorType::FileNotFound,
                                    t.position.clone(),
                                    format!("Could not find file `{}` ({})", file, e),
                                ))
                            }
                        },
                        TokenType::Identifier(file) => {
                            match fs::read_to_string(&format!("{}.ez", file)) {
                                Ok(contents) => {
                                    let mut new_tokens = lexer::lex(&contents, Rc::new(file))?;
                                    new_tokens.pop().unwrap();
                                    tokens.splice(i..=i + 1, new_tokens);
                                }
                                Err(e) => {
                                    return Err(Error::new(
                                        ErrorType::FileNotFound,
                                        t.position.clone(),
                                        format!("Could not find file `{}` ({})", file, e),
                                    ))
                                }
                            }
                        }
                        _ => {
                            return Err(Error::new(
                                ErrorType::SyntaxError,
                                t.position.clone(),
                                "Expected a filename after `use`".to_owned(),
                            ))
                        }
                    },
                },
                "replace" => {
                    let find = match tokens.get(i + 1).cloned() {
                        None => {
                            return Err(Error::new(
                                ErrorType::SyntaxError,
                                tokens[i].position.clone(),
                                "Expected find element `replace`".to_owned(),
                            ))
                        }
                        Some(t) => t,
                    };
                    let replace = match tokens.get(i + 2).cloned() {
                        None => {
                            return Err(Error::new(
                                ErrorType::SyntaxError,
                                tokens[i].position.clone(),
                                "Expected replace element `replace`".to_owned(),
                            ))
                        }
                        Some(t) => {
                            if let TokenType::String(s) = t.token_type {
                                lexer::lex(
                                    &s,
                                    Rc::new(format!(
                                        "{}/replace  at {}:{}",
                                        t.position.file, t.position.line_start, t.position.start
                                    )),
                                )?
                            } else {
                                vec![t]
                            }
                        }
                    };
                    tokens.drain(i..=i + 2);
                    for i in 0..tokens.len() {
                        if tokens[i] == find {
                            tokens.splice(i..=i, replace.clone());
                        }
                    }
                }
                "declare" => match tokens.get(i + 1).cloned() {
                    None => {
                        return Err(Error::new(
                            ErrorType::SyntaxError,
                            tokens[i].position.clone(),
                            "Expected an identifier after `declare`".to_owned(),
                        ))
                    }
                    Some(t) => match t.token_type {
                        TokenType::Identifier(ident) => {
                            declared.insert(ident);
                            tokens.drain(i..=i + 1);
                        }
                        _ => {
                            return Err(Error::new(
                                ErrorType::SyntaxError,
                                t.position.clone(),
                                "Expected an identifier after `declare`".to_owned(),
                            ))
                        }
                    },
                },
                "ifdeclared" => match tokens.get(i + 1).cloned() {
                    None => {
                        return Err(Error::new(
                            ErrorType::SyntaxError,
                            tokens[i].position.clone(),
                            "Expected an identifier after `declare`".to_owned(),
                        ))
                    }
                    Some(t) => match t.token_type {
                        TokenType::Identifier(ref ident) => {
                            if declared.contains(ident) {
                                ifs.push(None);
                            } else {
                                ifs.push(Some(i));
                            }
                            tokens.drain(i..=i + 1);
                        }
                        _ => {
                            return Err(Error::new(
                                ErrorType::SyntaxError,
                                t.position,
                                "Expected an identifier after `declare`".to_owned(),
                            ))
                        }
                    },
                },
                "else" => {
                    if let Some(idx) = ifs.last_mut() {
                        match idx {
                            Some(n) => {
                                tokens.drain(*n..=i);
                                *idx = None;
                            }
                            None => {
                                *idx = Some(i);
                                tokens.remove(i);
                            }
                        }
                    } else {
                        return Err(Error::new(
                            ErrorType::SyntaxError,
                            tokens[i].position.clone(),
                            "`else` without `ifdeclared`".to_owned(),
                        ));
                    }
                }
                "endif" => {
                    if let Some(idx) = ifs.pop() {
                        match idx {
                            Some(n) => {
                                tokens.drain(n..=i);
                            }

                            None => {
                                tokens.remove(i);
                            }
                        }
                    } else {
                        return Err(Error::new(
                            ErrorType::SyntaxError,
                            tokens[i].position.clone(),
                            "`endif` without `ifdeclared`".to_owned(),
                        ));
                    }
                }
                "error" => {
                    let msg = match tokens.get(i + 1).cloned() {
                        None => {
                            return Err(Error::new(
                                ErrorType::SyntaxError,
                                tokens[i].position.clone(),
                                "Expected an error message after `error`".to_owned(),
                            ))
                        }
                        Some(t) => match t.token_type {
                            TokenType::String(s) => s,
                            _ => {
                                return Err(Error::new(
                                    ErrorType::SyntaxError,
                                    t.position,
                                    "Expected an error message after `error`".to_owned(),
                                ))
                            }
                        },
                    };
                    tokens.drain(i..=i + 1);
                    if matches!(ifs.last(), Some(None) | None) {
                        return Err(Error::new(
                            ErrorType::PreprocessorError,
                            tokens[i].position.clone(),
                            msg,
                        ));
                    }
                }
                _ => unreachable!(),
            }
        } else {
            i += 1;
        }
    }

    if ifs.pop().is_some() {
        return Err(Error::new(
            ErrorType::SyntaxError,
            tokens[i - 1].position.clone(),
            "No `endif` after `ifdeclared`".to_owned(),
        ));
    }

    Ok(tokens)
}
