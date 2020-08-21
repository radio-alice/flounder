use std::path::Path;
use std::time::SystemTime;

static ALLOWED_EXTENSIONS: &[&'static str] = &[
    "gmi", "txt", "jpg", "jpeg", "gif", "png", "svg", "webp", "midi", "json", "csv", "gemini",
    "mp3",
];

pub fn ok_extension(filename: &str) -> bool {
    let tmp = filename.to_lowercase();
    let lower_extension: Option<&str> = Path::new(&tmp).extension().and_then(|s| s.to_str());
    return ALLOWED_EXTENSIONS
        .iter()
        .any(|s| Some(*s) == lower_extension);
}

// Probably could use a library here. Pointless optimization
pub fn rendered_time_ago(epoch_time: u32) -> String {
    // do some fun stuff
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let ago = now - epoch_time as u64;

    if ago < 60 {
        if ago == 1 {
            return "1 second ago".to_string();
        }
        else {
            return format!("{} seconds ago", ago);
        }
    } else if ago < 3600 {
        let minutes = ago / 60;
        if minutes == 1 {
            return "1 minute ago".to_string()
        } else {
            return format!("{} minutes ago", minutes);
        }
    } else if ago < 3600 * 24 {
        let hours = ago / 3600;
        if hours == 1 {
            return "1 hour ago".to_string()
        } else{
            return format!("{} hours ago", hours);
        }
    } else {
        let days = 3600 * 24;
        if days == 1 {
            return "1 day ago".to_string()
        } else {
            return format!("{} days ago", ago / days);
        }
    }
}
