extern crate libraptor_sys;

pub mod pull;
pub mod push;

use libraptor_sys::*;
use std::ffi::CString;
use std::fmt::Debug;
use std::mem;
use std::os::raw::c_char;
use std::os::raw::c_void;

#[derive(Debug)]
pub struct Statement {
    subject: Term,
    predicate: Term,
    object: Term,
    graph: Option<Term>,
}

#[derive(Debug)]
pub struct Literal {
    value: String,
    datatype: Option<IRI>,
    lang: Option<String>,
}

#[derive(Clone, Debug)]
pub struct IRI(String);

#[derive(Debug)]
pub enum Term {
    URI(IRI),
    Literal(Literal),
    Blank(String),
}

#[derive(Debug)]
pub enum LogLevel {
    None,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug)]
pub struct Locator {
    iri: Option<IRI>,
    file: Option<String>,
    line: Option<i32>,
    column: Option<i32>,
    byte: Option<i32>,
}

#[derive(Debug)]
pub struct LogMessage {
    text: String,
    level: LogLevel,
}

// libraptor strings are not normal strings -- they are Unicode
// strings null-terminated. In most cases, it's clear which one we are
// using -- either unicode buffer with a len, or a "proper" C
// string. However, some places mix the two. This method hopefully
// works.
fn raptor_string_to_rust_string(s: *const c_char) -> String {
    unsafe {
        // Count up to the null terminator
        let len = libc::strlen(s);

        // Convert from utf8
        std::str::from_utf8(std::slice::from_raw_parts(s as *mut u8, len as usize))
            .unwrap()
            .to_string()
    }
}

fn raptor_string_to_rust_string_maybe(s: *const c_char) -> Option<String> {
    if s.is_null() {
        None
    } else {
        Some(raptor_string_to_rust_string(s))
    }
}

#[allow(dead_code)]
fn raptor_locator_to_rust_locator(locator: *mut raptor_locator) -> Locator {
    unsafe {
        Locator {
            iri: if (*locator).uri.is_null() {
                None
            } else {
                Some(raptor_uri_to_rust_iri((*locator).uri))
            },
            file: raptor_string_to_rust_string_maybe((*locator).file),
            line: if (*locator).line < 0 {
                None
            } else {
                Some((*locator).line)
            },
            column: if (*locator).column < 0 {
                None
            } else {
                Some((*locator).column)
            },
            byte: if (*locator).byte < 0 {
                None
            } else {
                Some((*locator).byte)
            },
        }
    }
}

#[allow(non_upper_case_globals)]
fn raptor_log_message_to_rust_log_message(message: *mut raptor_log_message) -> LogMessage {
    unsafe {
        LogMessage {
            text: raptor_string_to_rust_string((*message).text),
            level: match (*message).level {
                raptor_log_level_RAPTOR_LOG_LEVEL_NONE => LogLevel::None,
                raptor_log_level_RAPTOR_LOG_LEVEL_TRACE => LogLevel::Trace,
                raptor_log_level_RAPTOR_LOG_LEVEL_DEBUG => LogLevel::Debug,
                raptor_log_level_RAPTOR_LOG_LEVEL_INFO => LogLevel::Info,
                raptor_log_level_RAPTOR_LOG_LEVEL_WARN => LogLevel::Warn,
                raptor_log_level_RAPTOR_LOG_LEVEL_ERROR => LogLevel::Error,
                raptor_log_level_RAPTOR_LOG_LEVEL_FATAL => LogLevel::Fatal,
                _ => panic!("Unknown log level"),
            },
        }
    }
}

fn raptor_statement_to_rust_statement(statement: *mut raptor_statement) -> Statement {
    unsafe {
        Statement {
            subject: raptor_term_to_rust_term((*statement).subject),
            predicate: raptor_term_to_rust_term((*statement).predicate),
            object: raptor_term_to_rust_term((*statement).object),
            graph: raptor_term_to_rust_term_maybe((*statement).graph),
        }
    }
}

fn raptor_uri_to_rust_iri(uri: *mut raptor_uri) -> IRI {
    unsafe {
        IRI(CString::from_raw(raptor_uri_to_string(uri) as *mut i8)
            .into_string()
            .unwrap())
    }
}

#[allow(non_upper_case_globals)]
fn raptor_term_to_rust_term(term: *mut raptor_term) -> Term {
    unsafe {
        match (*term).type_ {
            raptor_term_type_RAPTOR_TERM_TYPE_UNKNOWN => {
                panic!("Found raptor term with TERM_TYPE_UNKNOWN")
            }
            raptor_term_type_RAPTOR_TERM_TYPE_URI => {
                Term::URI(raptor_uri_to_rust_iri((*term).value.uri))
            }
            raptor_term_type_RAPTOR_TERM_TYPE_LITERAL => {
                Term::Literal(raptor_literal_to_rust_literal((*term).value.literal))
            }
            raptor_term_type_RAPTOR_TERM_TYPE_BLANK => Term::Blank(
                std::str::from_utf8(std::slice::from_raw_parts(
                    (*term).value.blank.string,
                    (*term).value.blank.string_len as usize,
                ))
                .unwrap()
                .to_string(),
            ),
            _ => {
                panic!("Found raptor term with unknown term type");
            }
        }
    }
}

fn raptor_term_to_rust_term_maybe(term: *mut raptor_term) -> Option<Term> {
    if term.is_null() {
        None
    } else {
        Some(raptor_term_to_rust_term(term))
    }
}

fn raptor_literal_to_rust_literal(literal: raptor_term_literal_value) -> Literal {
    unsafe {
        Literal {
            value: {
                std::str::from_utf8(std::slice::from_raw_parts(
                    literal.string,
                    literal.string_len as usize,
                ))
                .unwrap()
                .to_string()
            },
            datatype: {
                if literal.datatype.is_null() {
                    None
                } else {
                    Some(raptor_uri_to_rust_iri(literal.datatype))
                }
            },
            lang: {
                if literal.language.is_null() {
                    None
                } else {
                    Some(
                        std::str::from_utf8(std::slice::from_raw_parts(
                            literal.language,
                            literal.language_len as usize,
                        ))
                        .unwrap()
                        .to_string(),
                    )
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_new_free_world() {
        unsafe {
            let world = raptor_new_world();
            raptor_free_world(world);
        }
    }
}
