//! Connection String Validation and Building

use anyhow::{Result, anyhow};
use crate::tools::database::sql::types::{DatabaseConfig, DatabaseType};

/// Build connection string for different database types
pub fn build_connection_string(config: &DatabaseConfig) -> Result<String> {
    match config.database_type {
        DatabaseType::PostgreSQL => {
            if !config.connection_string.starts_with("postgresql://") &&
               !config.connection_string.starts_with("postgres://") {
                return Err(anyhow!("PostgreSQL connection string must start with postgresql:// or postgres://"));
            }
            Ok(config.connection_string.clone())
        }
        DatabaseType::MySQL => {
            if !config.connection_string.starts_with("mysql://") {
                return Err(anyhow!("MySQL connection string must start with mysql://"));
            }
            Ok(config.connection_string.clone())
        }
        DatabaseType::SQLite => {
            // SQLite can be a file path or memory database
            if config.connection_string == ":memory:" {
                Ok("sqlite::memory:".to_string())
            } else {
                Ok(format!("sqlite://{}", config.connection_string))
            }
        }
        DatabaseType::SqlServer => {
            if !config.connection_string.starts_with("sqlserver://") &&
               !config.connection_string.starts_with("mssql://") {
                return Err(anyhow!("SQL Server connection string must start with sqlserver:// or mssql://"));
            }
            Ok(config.connection_string.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgresql_connection_string() {
        let config = DatabaseConfig {
            database_type: DatabaseType::PostgreSQL,
            connection_string: "postgresql://user:pass@localhost/db".to_string(),
            max_connections: None,
            timeout: None,
        };

        let result = build_connection_string(&config).unwrap();
        assert_eq!(result, "postgresql://user:pass@localhost/db");
    }

    #[test]
    fn test_sqlite_connection_string() {
        let config = DatabaseConfig {
            database_type: DatabaseType::SQLite,
            connection_string: "/path/to/db.sqlite".to_string(),
            max_connections: None,
            timeout: None,
        };

        let result = build_connection_string(&config).unwrap();
        assert_eq!(result, "sqlite:///path/to/db.sqlite");
    }

    #[test]
    fn test_sqlite_memory_connection_string() {
        let config = DatabaseConfig {
            database_type: DatabaseType::SQLite,
            connection_string: ":memory:".to_string(),
            max_connections: None,
            timeout: None,
        };

        let result = build_connection_string(&config).unwrap();
        assert_eq!(result, "sqlite::memory:");
    }

    #[test]
    fn test_invalid_postgresql_connection_string() {
        let config = DatabaseConfig {
            database_type: DatabaseType::PostgreSQL,
            connection_string: "invalid://localhost".to_string(),
            max_connections: None,
            timeout: None,
        };

        assert!(build_connection_string(&config).is_err());
    }

    #[test]
    fn test_invalid_mysql_connection_string() {
        let config = DatabaseConfig {
            database_type: DatabaseType::MySQL,
            connection_string: "invalid://localhost".to_string(),
            max_connections: None,
            timeout: None,
        };

        assert!(build_connection_string(&config).is_err());
    }
}
