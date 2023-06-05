//! Assigning a distinct target directory to each test case doesn't scale.
//! Those directories are big and can easily fill up your disk (and they
//! have started to do so once the number of tests grew to 50+).
//!
//! This module provides functionality to re-use target directories across
//! different test cases.
//! We create N target directories, where N is the number of cores on the
//! machine. Each test case is assigned a target directory from this pool
//! once it kicks off.
//! Since the test runner doesn't execute more than N  test cases in
//! parallel, this approach guarantees that test cases don't have to wait
//! to be assigned a target directory (i.e. we still get maximum parallelism).
//!
//! We can't reuse the same target directory for all tests since each test
//! needs to acquire a unique lock on the target directory in order to
//! execute `cargo` commands. Having a single target directory would
//! serialize all tests, which would be a huge performance hit.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use object_pool::Reusable;

#[derive(Clone)]
pub struct TargetDirectoryPool {
    directories: Arc<object_pool::Pool<PathBuf>>,
}

impl TargetDirectoryPool {
    pub fn new(size: Option<usize>, test_env_dir: &Path) -> Self {
        let size = size.unwrap_or_else(|| num_cpus::get());
        // The function signature here is awkward because it doesn't allow us to
        // borrow from the environment in the closure, which in turns means we can't guarantee deterministic names for the target directories.
        // We work around the issue by attaching objects to the pool manually
        let directories = object_pool::Pool::new(0, || {
            panic!("Target directories should never be initialized via the init closure")
        });
        for i in 0..size {
            directories.attach(create_target_directory(test_env_dir, i));
        }
        Self {
            directories: Arc::new(directories),
        }
    }

    /// Pull a target directory from the pool.
    pub fn pull(&self) -> Reusable<'_, PathBuf> {
        self.directories
            .try_pull()
            .expect("Failed to pull a target directory from the pool")
    }
}

fn create_target_directory(test_env_dir: &Path, i: usize) -> PathBuf {
    let target_dir_path = test_env_dir
        .join("target_dirs")
        .join(format!("target_{:0>2}", i));
    fs_err::create_dir_all(&target_dir_path).expect("Failed to create target directory");
    target_dir_path
}
