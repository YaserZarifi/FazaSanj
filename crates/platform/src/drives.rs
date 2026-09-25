use fazasanj_model::{DriveInfo, DriveKind};

use crate::Result;

/// Every mounted drive letter with its label, filesystem and space.
#[cfg(windows)]
pub fn list_drives() -> Result<Vec<DriveInfo>> {
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };
    use windows_sys::Win32::System::WindowsProgramming::{
        DRIVE_CDROM, DRIVE_FIXED, DRIVE_NO_ROOT_DIR, DRIVE_REMOTE, DRIVE_REMOVABLE, DRIVE_UNKNOWN,
    };

    use crate::wide::{from_wide, to_wide};

    // SAFETY: no preconditions.
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return Err(crate::last_error("GetLogicalDrives"));
    }
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()).to_uppercase();

    let mut out = Vec::new();
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let letter = char::from(b'A' + i as u8);
        let root = format!("{letter}:\\");
        let root_w = to_wide(&root);

        // SAFETY: root_w is a valid null terminated string.
        let dtype = unsafe { GetDriveTypeW(root_w.as_ptr()) };
        let kind = match dtype {
            DRIVE_FIXED => DriveKind::Fixed,
            DRIVE_REMOVABLE => DriveKind::Removable,
            DRIVE_REMOTE => DriveKind::Network,
            DRIVE_CDROM | DRIVE_UNKNOWN | DRIVE_NO_ROOT_DIR => continue,
            _ => DriveKind::Other,
        };

        let mut label = [0u16; 261];
        let mut fs = [0u16; 261];
        // SAFETY: buffers are valid for the given lengths, optional outputs are null.
        let ok = unsafe {
            GetVolumeInformationW(
                root_w.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                fs.as_mut_ptr(),
                fs.len() as u32,
            )
        };
        if ok == 0 {
            // Not ready (empty card reader and so on).
            continue;
        }

        let mut free_to_caller = 0u64;
        let mut total = 0u64;
        let mut free = 0u64;
        // SAFETY: out pointers are valid u64s.
        let ok = unsafe { GetDiskFreeSpaceExW(root_w.as_ptr(), &mut free_to_caller, &mut total, &mut free) };
        if ok == 0 {
            continue;
        }

        let filesystem = from_wide(&fs);
        let letter_s = format!("{letter}:");
        out.push(DriveInfo {
            is_system: letter_s == system_drive,
            fast_scan_available: filesystem.eq_ignore_ascii_case("NTFS") && kind != DriveKind::Network,
            letter: letter_s,
            root,
            label: from_wide(&label),
            filesystem,
            total,
            free,
            kind,
        });
    }
    Ok(out)
}

#[cfg(not(windows))]
pub fn list_drives() -> Result<Vec<DriveInfo>> {
    Err(crate::PlatformError::Unsupported)
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn system_drive_is_listed() {
        let drives = super::list_drives().expect("list drives");
        let sys = drives.iter().find(|d| d.is_system).expect("system drive");
        assert!(sys.total > 0);
        assert!(sys.free <= sys.total);
    }
}
