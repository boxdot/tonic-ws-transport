pub mod channel {
    pub struct Channel;
    pub struct Endpoint;
}

pub mod server {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
