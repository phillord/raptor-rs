extern crate libraptor_sys;

use libraptor_sys::*;
use std::collections::VecDeque;
use std::mem;
use std::os::raw::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::marker::PhantomData;
use std::fmt::Debug;

pub struct World {
    raw: *mut raptor_world,
}

impl World {
    pub fn new() -> World {
        unsafe {
            World {
                raw: raptor_new_world(),
            }
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        unsafe {
            raptor_free_world(self.raw);
        }
    }
}

pub trait ParserHandler: Debug{
    fn handle_statement(&mut self, Statement) -> Result<(),String>;
    fn handle_error(&mut self, String) -> Result<(),String>;
}

#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyParserHandler{
}

impl ParserHandler for EmptyParserHandler{
    // Do these methods really need to return Result? Is there
    // anything we can do?
    fn handle_statement(&mut self, _statement:Statement) -> Result<(),String>
    {
        Ok(())
    }

    fn handle_error(&mut self, _:String) -> Result<(),String>
    {
        Ok(())
    }
}

#[derive(Debug)]
pub struct MemoryParserHandler(pub VecDeque<Result<Statement,String>>);

impl MemoryParserHandler {
    pub fn new() -> MemoryParserHandler {
        MemoryParserHandler(VecDeque::new())
    }
}

impl ParserHandler for MemoryParserHandler {
    fn handle_statement(&mut self, statement:Statement) -> Result<(),String>
    {
        self.0.push_back(Ok(statement));
        Ok(())
    }

    fn handle_error(&mut self, error:String) -> Result<(),String>
    {
        self.0.push_back(Err(error));
        Ok(())
    }
}

#[derive(Debug)]
pub struct Statement
{
    subject: Term,
    predicate: Term,
    object: Term,
    graph: Option<Term>
}

#[derive(Debug)]
pub struct Literal {
    value:String,
    datatype:Option<IRI>,
    lang:Option<String>,
}

#[derive(Clone, Debug)]
pub struct IRI(String);

#[derive(Debug)]
pub enum Term {
    URI(IRI),
    Literal(Literal),
    Blank(String)
}

pub struct Parser<'w>
{
    raw: *mut raptor_parser,
    marker: PhantomData<&'w World>,
    handler_ptr: *mut c_void
}

fn term_to_rust_string(term:*mut raptor_term) -> String {
    unsafe{
        let cstring:CString =
            CString::from_raw(raptor_term_to_string(term) as *mut i8);

        return cstring.into_string().unwrap();
    }
}

fn raptor_statement_to_rust_statement(statement:*mut raptor_statement) -> Statement
{
    unsafe{
        Statement{
            subject: raptor_term_to_rust_term((*statement).subject),
            predicate: raptor_term_to_rust_term((*statement).predicate),
            object: raptor_term_to_rust_term((*statement).object),
            graph: raptor_term_to_rust_term_maybe((*statement).graph)
        }
    }
}

fn raptor_uri_to_rust_iri(uri:*mut raptor_uri) -> IRI
{
    unsafe {
        IRI(CString::from_raw(raptor_uri_to_string(uri) as *mut i8).
            into_string().unwrap())
    }
}

#[allow(non_upper_case_globals)]
fn raptor_term_to_rust_term(term:*mut raptor_term) -> Term
{
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
            raptor_term_type_RAPTOR_TERM_TYPE_BLANK => {
                Term::Blank(
                    std::str::from_utf8(std::slice::from_raw_parts
                                        ((*term).value.blank.string,
                                         (*term).value.blank.string_len as usize))
                        .unwrap()
                        .to_string()
                )
            }
            _ => {
                panic!("Found raptor term with unknown term type");
            }
        }
    }
}

fn raptor_term_to_rust_term_maybe(term:*mut raptor_term)->Option<Term>
{
    if term.is_null() {
        None
    }
    else {
        Some(raptor_term_to_rust_term(term))
    }
}

fn raptor_literal_to_rust_literal(literal: raptor_term_literal_value) -> Literal
{
    unsafe {
        Literal{
            value: {
                std::str::from_utf8(std::slice::from_raw_parts
                                    (literal.string, literal.string_len as usize))
                    .unwrap()
                    .to_string()
            },
            datatype: {
                if literal.datatype.is_null() {
                    None
                }
                else {
                    Some(raptor_uri_to_rust_iri(literal.datatype))
                }
            },
            lang: {
                if literal.language.is_null() {
                    None
                }
                else {
                    Some(std::str::from_utf8(std::slice::from_raw_parts
                                             (literal.language, literal.language_len as usize))
                         .unwrap()
                         .to_string())
                }
            }
        }
    }
}

extern "C" fn statement_handler(user_data:*mut c_void,
                                statement: *mut raptor_statement)
{
    unsafe{
        let rust_statement =
            raptor_statement_to_rust_statement(statement);

        let ph:&mut Box<&mut ParserHandler> = mem::transmute(user_data);
        ph.handle_statement(rust_statement).ok();
    }
}

impl<'w> Parser<'w>{
    pub fn new(w: &mut World, kind: &str, baseuri: &str,
               handler: &ParserHandler,
    ) -> Parser<'w> {

        let kind = CString::new(kind).unwrap();
        let baseuri = CString::new(baseuri).unwrap();

        unsafe {
            let baseuri =
                raptor_new_uri(w.raw, baseuri.as_ptr() as *const u8);

            let double_boxed_handler: Box<Box<&ParserHandler>> = Box::new(Box::new(handler));
            let handler_ptr = Box::into_raw(double_boxed_handler) as *mut _;
            let parser =
                Parser {
                    raw: raptor_new_parser(w.raw, kind.as_ptr()),
                    marker: PhantomData,
                    handler_ptr
                };

            raptor_parser_set_statement_handler(parser.raw,
                                                handler_ptr,
                                                Some(statement_handler));
            raptor_parser_parse_start(parser.raw, baseuri);
            parser
        }
    }

    fn parse_cstr(&mut self, content: &CStr, size: usize) {
        unsafe { raptor_parser_parse_chunk(self.raw, content.as_ptr() as *const u8, size, 0); }
    }

    pub fn parse_chunk(&mut self, content: &str) {
        let len = content.len();
        let c = CString::new(content).unwrap();
        self.parse_cstr(&c, len)
    }

    pub fn parse_complete(&mut self) {
        unsafe{
            raptor_parser_parse_chunk(self.raw,std::ptr::null(), 0, 1);
        }
    }
}

impl<'w> Drop for Parser<'w>
{
    fn drop(&mut self) {
        unsafe {
            raptor_free_parser(self.raw);
            // Drop the handler
            let _: Box<Box<&ParserHandler>> = Box::from_raw(self.handler_ptr as *mut _);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Error;
    use std::ffi::OsStr;
    use std::path::Path;
    use std::fs::{read_dir};

    #[test]
    fn raw_new_free_world() {
        unsafe {
            let world = raptor_new_world();
            raptor_free_world(world);
        }
    }

    #[test]
    fn new_world() {
        World::new();
    }

    #[test]
    fn new_parser() {
        let mut w = World::new();
        let e = EmptyParserHandler::default();
        let _p = Parser::new(&mut w, "rdfxml", "http://www.example.com", &e);
    }

    #[test]
    fn test_empty_parse() {
        let mut w = World::new();
        let m = MemoryParserHandler::new();
        let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com", &m);
        p.parse_complete();
        assert_eq!(0, m.0.len());
    }

    #[test]
    fn test_parse() {
        let about = include_str!("./test-files/about.rdf");
        let mut w = World::new();
        let m = MemoryParserHandler::new();
        let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com", &m);
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_parse_with_lang() {
        let about = include_str!("./test-files/about_with_lang.rdf");
        let mut w = World::new();
        let m = MemoryParserHandler::new();
        let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com", &m);
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_two_parse() {
        let about = include_str!("./test-files/about_two.rdf");
        let mut w = World::new();
        let e = EmptyParserHandler::default();
        let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com", &e);
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_eph() {
        let _e = EmptyParserHandler::default();
    }

    #[test]
    fn test_convert_uri() -> Result<(),std::ffi::NulError> {
        unsafe{
            let w = raptor_new_world();
            let uri_string:CString = CString::new("http://www.example.com")?;
            let uri = raptor_new_uri(w, uri_string.as_ptr() as *const u8);
            let rust_uri = raptor_uri_to_rust_iri(uri);

            assert_eq!(rust_uri.0, "http://www.example.com");
            Ok(())
        }
    }

    #[test]
    fn w3c_test_suite() -> Result<(),Error> {
        // TODO Rewrite this out of copyright

        let rdf_ext = Some(OsStr::new("rdf"));
        let suite = Path::new("./").join("rdf-tests").join("rdf-xml");
        if !suite.exists() || !suite.is_dir() {
            panic!("rdf-tests/rdf-xml not found");
        }

        let test_files =
            read_dir(&suite).unwrap()
            .map(|subfile| subfile.ok().unwrap().path())
            .filter(|p| p.is_dir())
            .flat_map(|subdir| read_dir(subdir).unwrap())
            .map(|entry| entry.ok().unwrap().path())
            .filter(|path| path.extension() == rdf_ext);

        let mut count = 0;
        for path in test_files {

            let st = fs::read_to_string(path.clone())?;

            let mut w = World::new();
            let m = MemoryParserHandler::new();
            let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com", &m);
            p.parse_chunk(&st);
            p.parse_complete();

            count=count+1;

            let path = path.to_str().unwrap();
            if path.starts_with("error-") {
                assert!(
                    true
                    format!("{} should NOT parse without error", path)
                );
            } else {
                assert!(true, format!("{} should parse without error", path));
            }
        }
        assert_ne!(
            count, 0,
            "No test found in W3C test-suite, something must be wrong"
        );

        Ok(())
    }
}
