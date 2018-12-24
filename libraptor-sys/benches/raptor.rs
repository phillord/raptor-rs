#[macro_use]
extern crate criterion;
extern crate libraptor_sys;

use libraptor_sys::*;
use criterion::Criterion;
use std::ffi::CString;

fn new_world(){
    unsafe{
        let world = raptor_new_world_internal(RAPTOR_VERSION);
        raptor_free_world(world);
    }
}

fn new_world_bench(c: &mut Criterion){
    c.bench_function("new world",
                     |b| b.iter(|| new_world()));
}

unsafe fn read_all_char(rust_rdf:&str){
    let c_rdf = CString::new(rust_rdf).unwrap();
    let rdfxml = CString::new("rdfxml").unwrap();
    let base_uri = CString::new("http://www.example.com").unwrap();
    let world = raptor_new_world();
    raptor_world_open(world);

    let raptor_base_uri = raptor_new_uri(world,
                                         base_uri.as_bytes_with_nul()
                                         .as_ptr());

    let rdf_parser = raptor_new_parser(world, rdfxml.as_ptr());
    let _rtn = raptor_parser_parse_start(rdf_parser, raptor_base_uri);
    let _rtn = raptor_parser_parse_chunk(rdf_parser,
                                         c_rdf.as_bytes_with_nul().as_ptr(),
                                         c_rdf.as_bytes().len(),
                                         0);
    raptor_free_uri(raptor_base_uri);
    raptor_free_parser(rdf_parser);
    raptor_free_world(world)
}

fn read_all(){
    unsafe{
        let rust_rdf = include_str!("../src/test.rdf");
        read_all_char(rust_rdf);
    }
}

fn read_all_big(){
    unsafe {
        let rust_rdf = include_str!("../../src/test-files/go.owl");
        read_all_char(rust_rdf);
    }
}

unsafe fn read_single_char(rust_rdf:&str){
    let rdfxml = CString::new("rdfxml").unwrap();
    let base_uri = CString::new("http://www.example.com").unwrap();
    let world = raptor_new_world();
    raptor_world_open(world);

    let raptor_base_uri = raptor_new_uri(world,
                                         base_uri.as_bytes_with_nul()
                                         .as_ptr());

    let rdf_parser = raptor_new_parser(world, rdfxml.as_ptr());
    let _rtn = raptor_parser_parse_start(rdf_parser, raptor_base_uri);

    let b = rust_rdf.as_bytes();

    for i in 0 .. b.len(){
        let _rtn = raptor_parser_parse_chunk(rdf_parser,
                                             b[i..].as_ptr(),
                                             1,
                                             0);
    }
    raptor_parser_parse_chunk(rdf_parser,std::ptr::null(), 0, 1);

    raptor_free_uri(raptor_base_uri);
    raptor_free_parser(rdf_parser);
    raptor_free_world(world)
}

fn read_one_at_a_time(){
    unsafe {
        let rust_rdf = include_str!("../src/test.rdf");
        read_single_char(rust_rdf);
    }
}

fn read_big_one_at_a_time(){
    unsafe {
        let rust_rdf = include_str!("../../src/test-files/go.owl");
        read_single_char(rust_rdf);
    }
}

fn read_all_bench(c: &mut Criterion){
    c.bench_function("read_all",
                     |b| b.iter(||
                                read_all()));
}

fn read_one_at_a_time_bench(c: &mut Criterion){
    c.bench_function("read_one",
                     |b| b.iter(||
                                read_one_at_a_time()));
}

fn read_one_big_bench(c: &mut Criterion){

    c.bench_function("read_one_big",
                     |b| b.iter(||
                                read_big_one_at_a_time()));
}

fn read_all_big_bench(c: &mut Criterion){

    c.bench_function("read_all_big",
                     |b| b.iter(||
                                read_all_big()));
}

criterion_group!(benches, new_world_bench, read_all_bench,
                 read_one_at_a_time_bench);


fn slow_config() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group! {
    name = slow;
    config = slow_config();
    targets = read_one_big_bench, read_all_big_bench
}

criterion_main!(benches, slow);
