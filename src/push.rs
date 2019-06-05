use super::*;

use std::collections::VecDeque;
use std::ffi::CStr;
use std::ffi::CString;

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

pub struct Parser {
    raw: *mut raptor_parser,
    raw_world: *mut raptor_world,
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
