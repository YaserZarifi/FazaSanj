//! Category guesses for well-known places that no rule explains (used for coloring).

use fazasanj_model::Category;

const VIDEO_AUDIO_IMAGE: [&str; 22] = [
    "mp4", "mkv", "avi", "mov", "wmv", "webm", "m4v", "mp3", "flac", "wav", "m4a", "aac", "ogg", "opus", "jpg",
    "jpeg", "png", "heic", "raw", "cr2", "nef", "arw",
];
const VIRTUAL_DISKS: [&str; 5] = ["vhdx", "vhd", "vmdk", "vdi", "avhdx"];
const GAME_DIRS: [&str; 6] = ["steamapps", "epic games", "riot games", "xboxgames", "gog games", "ea games"];

/// Best guess for a path when no rule matched. `name` is the last component.
pub fn guess_category(path: &str, name: &str, is_dir: bool) -> Option<Category> {
    let path = path.strip_prefix("\\\\?\\").unwrap_or(path);
    let mut it = path.split(['\\', '/']).filter(|c| !c.is_empty());
    let _drive = it.next()?;
    let top = it.next();
    let second = it.next();
    let third = it.next();

    if !is_dir {
        if let Some((_, ext)) = name.rsplit_once('.') {
            if VIRTUAL_DISKS.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                return Some(Category::Virtualization);
            }
        }
    }
    let eq = |a: Option<&str>, b: &str| a.is_some_and(|x| x.eq_ignore_ascii_case(b));
    let has_game_dir = path.split(['\\', '/']).any(|c| GAME_DIRS.iter().any(|g| g.eq_ignore_ascii_case(c)));
    if has_game_dir {
        return Some(Category::Games);
    }

    let Some(top) = top else {
        return None;
    };
    let top_is = |s: &str| top.eq_ignore_ascii_case(s);
    if top_is("windows")
        || top_is("$recycle.bin")
        || top_is("system volume information")
        || top_is("recovery")
        || top_is("boot")
        || top_is("windows.old")
    {
        return Some(Category::System);
    }
    if second.is_none() && !is_dir {
        let n = name.to_ascii_lowercase();
        if matches!(n.as_str(), "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "dumpstack.log") {
            return Some(Category::System);
        }
    }
    if top_is("program files") || top_is("program files (x86)") || top_is("programdata") {
        return Some(Category::Apps);
    }
    if top_is("users") {
        let Some(_user) = second else {
            return Some(Category::UserFiles);
        };
        let Some(folder) = third else {
            return Some(Category::UserFiles);
        };
        if eq(Some(folder), "appdata") {
            return Some(Category::Apps);
        }
        for m in ["videos", "music", "pictures"] {
            if folder.eq_ignore_ascii_case(m) {
                return Some(Category::Media);
            }
        }
        if folder.eq_ignore_ascii_case("virtualbox vms") {
            return Some(Category::Virtualization);
        }
        if folder.starts_with('.') {
            return Some(Category::Dev);
        }
        if !is_dir && is_media_ext(name) {
            return Some(Category::Media);
        }
        return Some(Category::UserFiles);
    }
    if !is_dir && is_media_ext(name) {
        return Some(Category::Media);
    }
    None
}

fn is_media_ext(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, e)| VIDEO_AUDIO_IMAGE.iter().any(|m| m.eq_ignore_ascii_case(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_known_places() {
        let g = |p: &str, dir: bool| guess_category(p, p.rsplit('\\').next().unwrap_or(p), dir);
        assert_eq!(g("C:\\Windows\\System32", true), Some(Category::System));
        assert_eq!(g("T:\\Program Files\\App", true), Some(Category::Apps));
        assert_eq!(g("C:\\Users\\a\\Videos\\x", true), Some(Category::Media));
        assert_eq!(g("C:\\Users\\a\\Documents", true), Some(Category::UserFiles));
        assert_eq!(g("D:\\SteamLibrary\\steamapps\\common\\Game", true), Some(Category::Games));
        assert_eq!(g("C:\\Program Files (x86)\\Steam\\steamapps\\common", true), Some(Category::Games));
        assert_eq!(g("E:\\vm\\disk.vhdx", false), Some(Category::Virtualization));
        assert_eq!(g("E:\\random\\stuff", true), None);
        assert_eq!(g("C:\\pagefile.sys", false), Some(Category::System));
    }
}
