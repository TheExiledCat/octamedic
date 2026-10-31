pub trait Flip {
    /// Not `toggle`, so it can't clash with `bool::toggle`, which arrived after the Rust version
    /// this builds with.
    fn flip(&mut self);
}

impl Flip for bool {
    fn flip(&mut self) {
        *self = !*self;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flipping_twice_gets_back_where_it_started() {
        let mut value = false;
        value.flip();
        assert!(value);
        value.flip();
        assert!(!value);
    }
}
