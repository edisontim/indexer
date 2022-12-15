#[cfg(test)]
mod tests {
    use web3::types::H256;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        dbg!(format!["{:#x}", H256::zero()]);
    }
}
