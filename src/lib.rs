extern crate libraptor_sys;

#[cfg(test)]
mod tests {
    use libraptor_sys::*;

    #[test]
    fn new_world(){
        unsafe{
            let world = raptor_new_world();
            raptor_free_world(world);
        }
    }
}
