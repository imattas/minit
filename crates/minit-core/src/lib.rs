pub const PROJECT_NAME: &str = "minit";

#[cfg(test)]
mod tests {
    use super::PROJECT_NAME;

    #[test]
    fn project_name_is_minit() {
        assert_eq!(PROJECT_NAME, "minit");
    }
}
