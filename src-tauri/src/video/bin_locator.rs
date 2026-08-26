use tokio::process::Command;

pub fn get_ffmpeg_path() -> String {
    find_binary("ffmpeg")
}

pub fn get_ffprobe_path() -> String {
    find_binary("ffprobe")
}

fn find_binary(base_name: &str) -> String {
    #[cfg(target_os = "windows")]
    let exe_name = format!("{}.exe", base_name);
    #[cfg(not(target_os = "windows"))]
    let exe_name = base_name.to_string();

    // 1. Check directory of current executable
    if let Ok(curr_exe) = std::env::current_exe() {
        if let Some(parent) = curr_exe.parent() {
            let direct = parent.join(&exe_name);
            if direct.is_file() {
                return direct.to_string_lossy().to_string();
            }
            let in_resources = parent.join("resources").join(&exe_name);
            if in_resources.is_file() {
                return in_resources.to_string_lossy().to_string();
            }
            let in_bin = parent.join("bin").join(&exe_name);
            if in_bin.is_file() {
                return in_bin.to_string_lossy().to_string();
            }
        }
    }

    // 2. Check current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let direct = cwd.join(&exe_name);
        if direct.is_file() {
            return direct.to_string_lossy().to_string();
        }
        let in_resources = cwd.join("resources").join(&exe_name);
        if in_resources.is_file() {
            return in_resources.to_string_lossy().to_string();
        }
        let in_tauri_res = cwd.join("src-tauri").join("resources").join(&exe_name);
        if in_tauri_res.is_file() {
            return in_tauri_res.to_string_lossy().to_string();
        }
    }

    // 3. Fallback to system name
    base_name.to_string()
}

pub fn create_hidden_command(bin_path: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(bin_path);
    #[cfg(target_os = "windows")]
    {
        // 0x08000000 = CREATE_NO_WINDOW
        cmd.creation_flags(0x08000000);
    }
    cmd
}
