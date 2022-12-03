#[cfg(test)]
mod tests {
    use web3::types::H256;

    #[test]
    fn test_format() {
        let vec = vec!["one".to_string(), "two".to_string()];
        vec.iter().map(|s| s.as_ref());
    }
}
