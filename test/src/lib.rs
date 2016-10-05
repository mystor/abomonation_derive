#![feature(rustc_macro)]

#[macro_use]
extern crate abomonation_derive;
extern crate abomonation;

#[cfg(test)]
mod tests {
    use abomonation::*;

    #[derive(Eq, PartialEq, Abomonation)]
    pub struct Struct {
        a: String,
        b: u64,
        c: Vec<u8>,
    }

    #[test]
    fn test_struct() {
        // create some test data out of abomonation-approved types
        let record = Struct { a: "test".to_owned(), b: 0, c: vec![0, 1, 2] };

        // encode vector into a Vec<u8>
        let mut bytes = Vec::new();
        unsafe { encode(&record, &mut bytes); }

        // decode from binary data
        if let Some((result, rest)) = unsafe { decode::<Struct>(&mut bytes) } {
            assert!(result == &record);
            assert!(rest.len() == 0);
        }
    }

    #[derive(Eq, PartialEq, Abomonation)]
    pub struct EmptyStruct;

    #[test]
    fn test_empty_struct() {
        // create some test data out of abomonation-approved types
        let record = EmptyStruct;

        // encode vector into a Vec<u8>
        let mut bytes = Vec::new();
        unsafe { encode(&record, &mut bytes); }

        // decode from binary data
        if let Some((result, rest)) = unsafe { decode::<EmptyStruct>(&mut bytes) } {
            assert!(result == &record);
            assert!(rest.len() == 0);
        }
    }

    #[derive(Eq, PartialEq, Abomonation)]
    pub struct TupleStruct(String, u64, Vec<u8>);

    #[test]
    fn test_tuple_struct() {
        // create some test data out of abomonation-approved types
        let record = TupleStruct("test".to_owned(), 0, vec![0, 1, 2]);

        // encode vector into a Vec<u8>
        let mut bytes = Vec::new();
        unsafe { encode(&record, &mut bytes); }

        // decode from binary data
        if let Some((result, rest)) = unsafe { decode::<TupleStruct>(&mut bytes) } {
            assert!(result == &record);
            assert!(rest.len() == 0);
        }
    }
}
