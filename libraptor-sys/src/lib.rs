#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// This is some weird macro thing in C.
pub unsafe fn raptor_new_world() -> *mut raptor_world {
    raptor_new_world_internal(RAPTOR_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn new_world_internal() {
        unsafe{
            let world = raptor_new_world_internal(RAPTOR_VERSION);
            raptor_free_world(world);
        }
        assert!(true);
    }

    #[test]
    fn new_world() {
        unsafe{
            let world = raptor_new_world();
            raptor_world_open(world);
            raptor_free_world(world);
        }
        assert!(true);

    }

    #[test]
    fn prepare_parser() {
        unsafe{
            let world = raptor_new_world();

            let rdfxml = CString::new("rdfxml").unwrap();
            let rdf_parser = raptor_new_parser(world, rdfxml.as_ptr());
            raptor_free_parser(rdf_parser);
            raptor_free_world(world);
        }
    }

    #[test]
    fn parser() {

        unsafe{
            let rust_rdf = include_str!("./test.rdf");
            let c_rdf = CString::new(rust_rdf).unwrap();

            let world = raptor_new_world();
            raptor_world_open(world);
            let rdfxml = CString::new("rdfxml").unwrap();
            let base_uri = CString::new("http://www.example.com").unwrap();
            let raptor_base_uri = raptor_new_uri(world,
                                                 base_uri.as_bytes_with_nul()
                                                 .as_ptr());
            let rdf_parser = raptor_new_parser(world, rdfxml.as_ptr());
            let rtn = raptor_parser_parse_start(rdf_parser, raptor_base_uri);
            assert_eq!(rtn, 0);
            let rtn = raptor_parser_parse_chunk(rdf_parser,
                                                c_rdf.as_bytes_with_nul().as_ptr(),
                                                c_rdf.as_bytes().len(),
                                                0);

            assert_eq!(rtn, 0);

            raptor_free_world(world);
        }
    }
}
