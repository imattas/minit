use minit_core::manager::{MountPlan, SwapPlan};
use thiserror::Error;

#[cfg(target_os = "linux")]
const SWAP_FLAG_PREFER: i32 = 0x8000;
#[cfg(target_os = "linux")]
const SWAP_FLAG_PRIO_SHIFT: i32 = 0;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("mount {target} failed: {message}")]
    Mount { target: String, message: String },
    #[error("unmount {target} failed: {message}")]
    Unmount { target: String, message: String },
    #[error("swapon {path} failed: {message}")]
    SwapOn { path: String, message: String },
    #[error("swapoff {path} failed: {message}")]
    SwapOff { path: String, message: String },
}

pub trait StorageExecutor {
    fn mount(&mut self, plan: &MountPlan) -> Result<(), StorageError>;
    fn unmount(&mut self, plan: &MountPlan) -> Result<(), StorageError>;
    fn swap_on(&mut self, plan: &SwapPlan) -> Result<(), StorageError>;
    fn swap_off(&mut self, plan: &SwapPlan) -> Result<(), StorageError>;
}

#[derive(Default)]
pub struct SystemStorageExecutor;

#[cfg(target_os = "linux")]
impl StorageExecutor for SystemStorageExecutor {
    fn mount(&mut self, plan: &MountPlan) -> Result<(), StorageError> {
        use nix::mount::{mount, MsFlags};

        std::fs::create_dir_all(&plan.where_path).map_err(|error| StorageError::Mount {
            target: plan.where_path.clone(),
            message: error.to_string(),
        })?;
        let mut flags = MsFlags::empty();
        let mut data_options = Vec::new();
        for option in &plan.options {
            match option.as_str() {
                "nodev" => flags |= MsFlags::MS_NODEV,
                "noexec" => flags |= MsFlags::MS_NOEXEC,
                "nosuid" => flags |= MsFlags::MS_NOSUID,
                "ro" => flags |= MsFlags::MS_RDONLY,
                _ => data_options.push(option.as_str()),
            }
        }
        let options = match data_options.is_empty() {
            true => None,
            false => Some(data_options.join(",")),
        };
        mount(
            Some(plan.what.as_str()),
            plan.where_path.as_str(),
            Some(plan.fstype.as_str()),
            flags,
            options.as_deref(),
        )
        .map_err(|error| StorageError::Mount {
            target: plan.where_path.clone(),
            message: error.to_string(),
        })
    }

    fn unmount(&mut self, plan: &MountPlan) -> Result<(), StorageError> {
        nix::mount::umount(plan.where_path.as_str()).map_err(|error| StorageError::Unmount {
            target: plan.where_path.clone(),
            message: error.to_string(),
        })
    }

    fn swap_on(&mut self, plan: &SwapPlan) -> Result<(), StorageError> {
        let path =
            std::ffi::CString::new(plan.path.as_str()).map_err(|error| StorageError::SwapOn {
                path: plan.path.clone(),
                message: error.to_string(),
            })?;
        let flags = plan.priority.map_or(0, |priority| {
            SWAP_FLAG_PREFER | ((priority as i32) << SWAP_FLAG_PRIO_SHIFT)
        });
        let result = unsafe { libc::swapon(path.as_ptr(), flags) };
        if result == 0 {
            Ok(())
        } else {
            Err(StorageError::SwapOn {
                path: plan.path.clone(),
                message: std::io::Error::last_os_error().to_string(),
            })
        }
    }

    fn swap_off(&mut self, plan: &SwapPlan) -> Result<(), StorageError> {
        let path =
            std::ffi::CString::new(plan.path.as_str()).map_err(|error| StorageError::SwapOff {
                path: plan.path.clone(),
                message: error.to_string(),
            })?;
        let result = unsafe { libc::swapoff(path.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(StorageError::SwapOff {
                path: plan.path.clone(),
                message: std::io::Error::last_os_error().to_string(),
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
impl StorageExecutor for SystemStorageExecutor {
    fn mount(&mut self, _plan: &MountPlan) -> Result<(), StorageError> {
        Ok(())
    }

    fn unmount(&mut self, _plan: &MountPlan) -> Result<(), StorageError> {
        Ok(())
    }

    fn swap_on(&mut self, _plan: &SwapPlan) -> Result<(), StorageError> {
        Ok(())
    }

    fn swap_off(&mut self, _plan: &SwapPlan) -> Result<(), StorageError> {
        Ok(())
    }
}
