#[cfg(test)]
mod tests {
    use crate::fuck;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_fuck() {
        fuck(true);
    }
}

pub fn fuck(a: bool) {
    if a {
        println!("fucK");
    } else {
        println!("fucK you");
    }
}
