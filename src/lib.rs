extern crate libraptor_sys;

use libraptor_sys::*;
use std::mem;
use std::os::raw::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::marker::PhantomData;
use std::fmt::Debug;

fn check_null() {}

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
    fn handle_statement(&mut self, String) -> Result<(),String>;
    fn handle_error(&self, String) -> Result<(),String>;
}

#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyParserHandler{
}

impl ParserHandler for EmptyParserHandler{
    fn handle_statement(&mut self, _:String) -> Result<(),String>
    {
        Ok(())
    }

    fn handle_error(&self, _:String) -> Result<(),String>
    {
        Ok(())
    }
}

pub struct Parser<'w>
{
    pub raw: *mut raptor_parser,
    pub marker: PhantomData<&'w World>,
    handler_ptr: *mut c_void
}



fn term_to_rust_string(term:*mut raptor_term) -> String {
    unsafe{
        let cstring:CString =
            CString::from_raw(raptor_term_to_string(term) as *mut i8);

        return cstring.into_string().unwrap();
    }
}

extern "C" fn statement_handler(user_data:*mut c_void,
                                statement: *mut raptor_statement)
{
    unsafe{
        let ph:&mut Box<ParserHandler> = mem::transmute(user_data);
        ph.handle_statement("hello".to_string()).ok();

    }
}

impl<'w> Parser<'w>{
    pub fn new(w: &mut World, kind: &str, baseuri: &str,
               handler: Box<ParserHandler>,
    ) -> Parser<'w> {

        let kind = CString::new(kind).unwrap();
        let baseuri = CString::new(baseuri).unwrap();

        unsafe {
            let baseuri =
                raptor_new_uri(w.raw, baseuri.as_ptr() as *const u8);

            let double_boxed_handler: Box<Box<ParserHandler>> = Box::new(handler);
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
            let _: Box<Box<ParserHandler>> = Box::from_raw(self.handler_ptr as *mut _);
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

    #[test]
    fn new_world() {
        World::new();
    }

    #[test]
    fn new_parser() {
        let mut w = World::new();
        let e = EmptyParserHandler::default();
        let _p = Parser::new(&mut w, "rdfxml", "http://www.example.com",
                             Box::new(e));
    }

    #[test]
    fn test_empty_parse() {
        let mut w = World::new();
        let e = EmptyParserHandler::default();
        let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com",
                                Box::new(e));
        p.parse_complete();
    }

    #[test]
    fn test_parse() {
        let about = include_str!("./test-files/about.rdf");
        let mut w = World::new();
        let e = EmptyParserHandler::default();
        let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com",
                                Box::new(e));
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_two_parse() {
        let about = include_str!("./test-files/about_two.rdf");
        let mut w = World::new();
        let e = EmptyParserHandler::default();
        let mut p = Parser::new(&mut w, "rdfxml", "http://www.example.com",
                                Box::new(e));
        p.parse_chunk(about);
        p.parse_complete();
    }

    #[test]
    fn test_eph() {
        let _e = EmptyParserHandler::default();
    }

}
