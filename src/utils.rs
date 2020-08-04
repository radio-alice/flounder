use std::path::Path;
use std::time::SystemTime;

static ALLOWED_EXTENSIONS: &[&'static str] = &["gmi", "txt", "jpg", "jpeg", "gif", "ping", "midi", "json", "csv", "gemini", "mp3"];

pub fn ok_extension(filename: &str) -> bool {
    let tmp = filename.to_lowercase();
    let lower_extension: Option<&str> = Path::new(&tmp).extension().and_then(|s| s.to_str());
    return ALLOWED_EXTENSIONS.iter().any(|s| Some(*s) == lower_extension)
    }

pub fn rendered_time_ago(epoch_time: u32) -> String {
    // do some fun stuff
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let ago = now - epoch_time as u64;

    if ago < 60 {
        return format!("{} seconds ago", ago);
    } else if ago < 3600 {
        return format!("{} minutes ago", ago / 60);
    } else if ago < 3600 * 24 {
        return format!("{} hours ago", ago / 3600);
    } else {
        return format!("{} days ago", ago / (3600 * 24));
    }
}
