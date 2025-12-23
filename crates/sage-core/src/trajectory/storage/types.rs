//! Common types for trajectory storage

/// Statistics about trajectory storage.
///
/// Provides metrics about stored trajectories including count and size information.
///
/// # Examples
///
/// ```no_run
/// use sage_core::trajectory::storage::{TrajectoryStorage, FileStorage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = FileStorage::new("trajectories")?;
/// let stats = storage.statistics().await?;
///
/// println!("Total trajectories: {}", stats.total_records);
/// println!("Total size: {} bytes", stats.total_size_bytes);
/// println!("Average size: {} bytes", stats.average_record_size);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct StorageStatistics {
    /// Total number of stored trajectories.
    pub total_records: usize,
    /// Total storage size in bytes.
    pub total_size_bytes: u64,
    /// Average record size in bytes.
    pub average_record_size: u64,
}

/// Rotation configuration for trajectory files.
///
/// Controls automatic deletion of old trajectory files to prevent
/// unbounded storage growth. You can limit by count, total size, or both.
///
/// # Examples
///
/// ```no_run
/// use sage_core::trajectory::storage::{RotationConfig, FileStorage};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Keep only the 10 most recent trajectories
/// let config = RotationConfig::with_max_trajectories(10);
/// let storage = FileStorage::with_config("trajectories", false, config)?;
///
/// // Limit total storage to 100MB
/// let config = RotationConfig::with_total_size_limit(100 * 1024 * 1024);
/// let storage = FileStorage::with_config("trajectories", false, config)?;
///
/// // Apply both limits
/// let config = RotationConfig::with_limits(50, 500 * 1024 * 1024);
/// let storage = FileStorage::with_config("trajectories", false, config)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct RotationConfig {
    /// Maximum number of trajectory files to keep.
    ///
    /// When exceeded, oldest files are deleted. `None` means unlimited.
    pub max_trajectories: Option<usize>,

    /// Maximum total size in bytes for all trajectories.
    ///
    /// When exceeded, oldest files are deleted until under limit. `None` means unlimited.
    pub total_size_limit: Option<u64>,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            max_trajectories: None,
            total_size_limit: None,
        }
    }
}

impl RotationConfig {
    /// Create a rotation config with max trajectories limit.
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum number of trajectory files to keep
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::RotationConfig;
    ///
    /// let config = RotationConfig::with_max_trajectories(20);
    /// assert_eq!(config.max_trajectories, Some(20));
    /// assert_eq!(config.total_size_limit, None);
    /// ```
    pub fn with_max_trajectories(max: usize) -> Self {
        Self {
            max_trajectories: Some(max),
            total_size_limit: None,
        }
    }

    /// Create a rotation config with total size limit.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum total size in bytes for all trajectories
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::RotationConfig;
    ///
    /// let config = RotationConfig::with_total_size_limit(1024 * 1024 * 100); // 100MB
    /// assert_eq!(config.max_trajectories, None);
    /// assert_eq!(config.total_size_limit, Some(1024 * 1024 * 100));
    /// ```
    pub fn with_total_size_limit(limit: u64) -> Self {
        Self {
            max_trajectories: None,
            total_size_limit: Some(limit),
        }
    }

    /// Create a rotation config with both count and size limits.
    ///
    /// Both limits are enforced - whichever is exceeded first triggers rotation.
    ///
    /// # Arguments
    ///
    /// * `max_trajectories` - Maximum number of trajectory files
    /// * `total_size_limit` - Maximum total size in bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use sage_core::trajectory::storage::RotationConfig;
    ///
    /// let config = RotationConfig::with_limits(50, 200 * 1024 * 1024); // 50 files or 200MB
    /// assert_eq!(config.max_trajectories, Some(50));
    /// assert_eq!(config.total_size_limit, Some(200 * 1024 * 1024));
    /// ```
    pub fn with_limits(max_trajectories: usize, total_size_limit: u64) -> Self {
        Self {
            max_trajectories: Some(max_trajectories),
            total_size_limit: Some(total_size_limit),
        }
    }
}
