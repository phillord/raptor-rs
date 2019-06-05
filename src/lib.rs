extern crate libraptor_sys;

use libraptor_sys::*;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::ffi::CString;
use std::fmt::Debug;
use std::mem;
use std::os::raw::c_char;
use std::os::raw::c_void;

pub trait ParserHandler: Debug {
    fn handle_statement(&mut self, Statement) -> Result<(), String>;
    fn handle_error(&mut self, LogMessage) -> Result<(), String>;
}

#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyParserHandler {}

impl ParserHandler for EmptyParserHandler {
    // Do these methods really need to return Result? Is there
    // anything we can do?
    fn handle_statement(&mut self, _statement: Statement) -> Result<(), String> {
        Ok(())
    }

    fn handle_error(&mut self, _: LogMessage) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct MemoryParserHandler(pub VecDeque<Result<Statement, LogMessage>>);

impl MemoryParserHandler {
    pub fn new() -> MemoryParserHandler {
        MemoryParserHandler(VecDeque::new())
    }
}

impl ParserHandler for MemoryParserHandler {
    fn handle_statement(&mut self, statement: Statement) -> Result<(), String> {
        self.0.push_back(Ok(statement));
        Ok(())
    }

    fn handle_error(&mut self, error: LogMessage) -> Result<(), String> {
        self.0.push_back(Err(error));
        Ok(())
    }
}

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

pub struct Parser {
    raw: *mut raptor_parser,
    raw_world: *mut raptor_world,
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

extern "C" fn log_handler(user_data: *mut c_void, message: *mut raptor_log_message) {
    unsafe {
        let ph: &mut Box<&mut ParserHandler> = mem::transmute(user_data);
        let rust_log_message = raptor_log_message_to_rust_log_message(message);
        ph.handle_error(rust_log_message).ok();
    }
}

extern "C" fn statement_handler(user_data: *mut c_void, statement: *mut raptor_statement) {
    unsafe {
        let rust_statement = raptor_statement_to_rust_statement(statement);

        let ph: &mut Box<&mut ParserHandler> = mem::transmute(user_data);
        ph.handle_statement(rust_statement).ok();
    }
}

impl<'w> Parser {
    pub fn new(kind: &str, baseuri: &str, handler: &ParserHandler) -> Parser {
        let kind = CString::new(kind).unwrap();
        let baseuri = CString::new(baseuri).unwrap();

        unsafe {
            let world = raptor_new_world();

            let baseuri = raptor_new_uri(world, baseuri.as_ptr() as *const u8);

            let double_boxed_handler: Box<Box<&ParserHandler>> = Box::new(Box::new(handler));
            let handler_ptr = Box::into_raw(double_boxed_handler) as *mut _;

            let parser = Parser {
                raw: raptor_new_parser(world, kind.as_ptr()),
                raw_world: world,
            };

            raptor_world_set_log_handler(parser.raw_world, handler_ptr, Some(log_handler));

            raptor_parser_set_statement_handler(parser.raw, handler_ptr, Some(statement_handler));

            raptor_parser_parse_start(parser.raw, baseuri);
            parser
        }
    }

    fn parse_cstr(&mut self, content: &CStr, size: usize) {
        unsafe {
            raptor_parser_parse_chunk(self.raw, content.as_ptr() as *const u8, size, 0);
        }
    }

    pub fn parse_chunk(&mut self, content: &str) {
        let len = content.len();
        let c = CString::new(content).unwrap();
        self.parse_cstr(&c, len)
    }

    pub fn parse_complete(&mut self) {
        unsafe {
            raptor_parser_parse_chunk(self.raw, std::ptr::null(), 0, 1);
        }
    }
}

impl<'w> Drop for Parser {
    fn drop(&mut self) {
        unsafe {
            raptor_free_parser(self.raw);
            raptor_free_world(self.raw_world);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::fs;
    use std::fs::read_dir;
    use std::io::Error;
    use std::path::Path;

    #[test]
    fn raw_new_free_world() {
        unsafe {
            let world = raptor_new_world();
            raptor_free_world(world);
        }
    }

    #[test]
    fn new_parser() {
        let e = EmptyParserHandler::default();
        let _p = Parser::new("rdfxml", "http://www.example.com", &e);
    }

    #[test]
    fn test_empty_parse() {
        let m = MemoryParserHandler::new();
        let mut p = Parser::new("rdfxml", "http://www.example.com", &m);
        p.parse_complete();

        // one error!
        assert_eq!(1, m.0.len());
    }

    #[test]
    fn test_parse() {
        let about = include_str!("./test-files/about.rdf");
        let m = MemoryParserHandler::new();
        let mut p = Parser::new("rdfxml", "http://www.example.com", &m);
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_parse_with_lang() {
        let about = include_str!("./test-files/about_with_lang.rdf");
        let m = MemoryParserHandler::new();
        let mut p = Parser::new("rdfxml", "http://www.example.com", &m);
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_two_parse() {
        let about = include_str!("./test-files/about_two.rdf");
        let e = EmptyParserHandler::default();
        let mut p = Parser::new("rdfxml", "http://www.example.com", &e);
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_eph() {
        let _e = EmptyParserHandler::default();
    }

    #[test]
    fn test_convert_uri() -> Result<(), std::ffi::NulError> {
        unsafe {
            let w = raptor_new_world();
            let uri_string: CString = CString::new("http://www.example.com")?;
            let uri = raptor_new_uri(w, uri_string.as_ptr() as *const u8);
            let rust_uri = raptor_uri_to_rust_iri(uri);

            assert_eq!(rust_uri.0, "http://www.example.com");

            raptor_free_uri(uri);
            raptor_free_world(w);
            Ok(())
        }
    }

    #[test]
    fn w3c_test_suite() -> Result<(), Error> {
        // TODO Rewrite this out of copyright

        let rdf_ext = Some(OsStr::new("rdf"));
        let suite = Path::new("./").join("rdf-tests").join("rdf-xml");
        if !suite.exists() || !suite.is_dir() {
            panic!("rdf-tests/rdf-xml not found");
        }

        let test_files = read_dir(&suite)
            .unwrap()
            .map(|subfile| subfile.ok().unwrap().path())
            .filter(|p| p.is_dir())
            .flat_map(|subdir| read_dir(subdir).unwrap())
            .map(|entry| entry.ok().unwrap().path())
            .filter(|path| path.extension() == rdf_ext);

        let mut count = 0;
        for path in test_files {
            let st = fs::read_to_string(path.clone())?;

            let m = MemoryParserHandler::new();
            let mut p = Parser::new("rdfxml", "http://www.example.com", &m);
            p.parse_chunk(&st);
            p.parse_complete();

            count = count + 1;

            let path = path.file_name().unwrap().to_str().unwrap();

            if path.starts_with("error") || path.starts_with("warn") {
                assert!(
                    m.0.iter().any(|event| event.is_err()),
                    format!("{} should NOT parse without error", path)
                );
            } else {
                assert!(
                    m.0.iter().all(|event| event.is_ok()),
                    format!("{} should parse without error", path)
                );
            }
        }
        assert_ne!(
            count, 0,
            "No test found in W3C test-suite, something must be wrong"
        );

        Ok(())
    }
}
