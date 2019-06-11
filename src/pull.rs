use super::push::Parser as PullParser;
use super::*;
use std::io::BufRead;

pub struct Parser<'w> {
    pull: PullParser<'w>,
}

impl<'w> Parser<'w> {
    pub fn new() -> Parser<'static> {
        unimplemented!()
    }

    pub fn parse_bufreader<R: BufRead>(&mut self, _bufreader: &R) -> Result<(), String> {
        unimplemented!()
    }
    pub fn parse_str(&mut self, _content: &str) -> Result<(), String> {
        unimplemented!()
    }

    //pub fn parse_file(p: Path) {}

    pub fn read_event() -> Result<Statement, LogMessage> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn new_parser() {
        assert!(true);
    }
}
