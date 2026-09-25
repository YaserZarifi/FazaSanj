//! Built-in category guess from well known folder names and extensions. Only used where the
//! knowledge base did not tag anything, so it stays small and conservative.

use fazasanj_model::Category;

/// Category for a folder, given its lowercase name, its depth below a drive root (None when
/// the scan root is not a drive root) and what its parent was guessed as.
pub fn guess_dir(name: &str, drive_depth: Option<u32>, inherited: Category) -> Category {
    if drive_depth == Some(1) {
        match name {
            "windows" | "system volume information" | "recovery" | "$windows.~bt" | "$windows.~ws" | "windows.old" => {
                return Category::System;
            }
            "program files" | "program files (x86)" | "programdata" => return Category::Apps,
            "users" => return Category::UserFiles,
            _ => {}
        }
    }
    // Specific kinds win over broad areas like apps or user files.
    let specific = match name {
        "steamapps" | "steamlibrary" | "epic games" | "xboxgames" | "riot games" | "gog galaxy" | "battle.net"
        | "ubisoft game launcher" | "ea games" => Some(Category::Games),
        "node_modules" | ".git" | ".gradle" | ".m2" | ".cargo" | ".rustup" | ".nuget" | "__pycache__" | ".venv"
        | ".npm" | ".pnpm-store" | ".android" | ".conda" | "anaconda3" | "miniconda3" => Some(Category::Dev),
        "temp" | "tmp" | "cache" | "caches" | "inetcache" | "code cache" | "gpucache" | "shadercache"
        | "dxcache" | "d3dscache" | "nvidia corporation" | "crashdumps" => Some(Category::Cache),
        "virtual machines" | "virtualbox vms" | "hyper-v" | "wsl" => Some(Category::Virtualization),
        "telegram desktop" | "whatsapp" | "discord" | "slack" | "microsoft teams" | "viber" => {
            Some(Category::Messaging)
        }
        "chrome" | "firefox" | "brave-browser" | "vivaldi" | "opera software" | "mozilla" => {
            Some(Category::Browsers)
        }
        _ => None,
    };
    if let Some(c) = specific {
        return c;
    }
    if inherited == Category::Unknown || inherited == Category::UserFiles {
        match name {
            "documents" | "desktop" | "downloads" | "pictures" | "videos" | "music" | "onedrive" => {
                return Category::UserFiles;
            }
            _ => {}
        }
    }
    inherited
}

/// Category for a file, given its lowercase name and its folder's category.
pub fn guess_file(name: &str, inherited: Category) -> Category {
    match name {
        "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" => return Category::System,
        _ => {}
    }
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    match ext {
        "vhd" | "vhdx" | "vmdk" | "vdi" | "qcow2" | "avhdx" => return Category::Virtualization,
        _ => {}
    }
    if inherited == Category::Unknown || inherited == Category::UserFiles {
        let media = matches!(
            ext,
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" | "m4v" | "jpg" | "jpeg" | "png" | "gif" | "heic"
                | "webp" | "raw" | "cr2" | "nef" | "mp3" | "flac" | "wav" | "m4a" | "aac" | "ogg"
        );
        if media {
            return Category::Media;
        }
    }
    inherited
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_level_folders() {
        assert_eq!(guess_dir("windows", Some(1), Category::Unknown), Category::System);
        assert_eq!(guess_dir("windows", Some(3), Category::Unknown), Category::Unknown);
        assert_eq!(guess_dir("program files", Some(1), Category::Unknown), Category::Apps);
    }

    #[test]
    fn specific_beats_broad() {
        assert_eq!(guess_dir("steamapps", Some(3), Category::Apps), Category::Games);
        assert_eq!(guess_dir("node_modules", Some(5), Category::UserFiles), Category::Dev);
        assert_eq!(guess_dir("temp", Some(2), Category::System), Category::Cache);
        assert_eq!(guess_dir("anything", Some(2), Category::Games), Category::Games);
    }

    #[test]
    fn files() {
        assert_eq!(guess_file("movie.mkv", Category::UserFiles), Category::Media);
        assert_eq!(guess_file("icon.png", Category::Apps), Category::Apps);
        assert_eq!(guess_file("disk.vhdx", Category::UserFiles), Category::Virtualization);
        assert_eq!(guess_file("pagefile.sys", Category::Unknown), Category::System);
    }
}
