use amaranctl::sqlite_dyn::extract_credentials_from_db;
use std::path::Path;

#[test]
fn test_dynamic_sqlite_not_crashing_on_invalid_db() {
    let result = extract_credentials_from_db(Path::new("non_existent_database_file.db"));
    assert!(result.is_err());
}
