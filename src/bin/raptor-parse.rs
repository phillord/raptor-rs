extern crate clap;
#[macro_use]
extern crate failure;
extern crate raptor_rs;

use clap::App;
use clap::Arg;

use failure::Error;

use std::fs;

use raptor_rs::*;

#[derive(Debug, Fail)]
pub enum CommandError {
    #[fail(display = "An argument that was expected is missing")]
    MissingArgument,
}

fn main() -> Result<(), Error> {
    //Hello
    let matches = App::new("raptor-parse")
        .version("0.1")
        .about("Parse an RDF file")
        .author("Phillip Lord")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1),
        )
        .get_matches();

    let input = matches
        .value_of("INPUT")
        .ok_or(CommandError::MissingArgument)?;

    let m = MemoryParserHandler::new();
    let mut p = Parser::new("rdfxml", "http://www.example.com", &m);

    let st = fs::read_to_string(input)?;

    p.parse_chunk(&st);
    p.parse_complete();

    println!("Parsed {} events", m.0.len());
    Ok(())
}
