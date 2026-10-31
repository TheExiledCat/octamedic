pub trait Flip {
    // not `toggle`, newer rust has bool::toggle and it'd clash
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
    fn flip_twice() {
        let mut value = false;
        value.flip();
        assert!(value);
        value.flip();
        assert!(!value);
    }
}
