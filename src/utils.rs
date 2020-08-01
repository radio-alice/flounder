use std::time::SystemTime;

pub fn rendered_time_ago(epoch_time: u32) -> String {
    // do some fun stuff
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let ago = now - epoch_time as u64;

    if ago < 60 {
        return format!("{} seconds ago", ago)
    } else if ago < 3600 {
        return format!("{} minutes ago", ago / 60)
    } else if ago < 3600 * 24 {
        return format!("{} hours ago", ago / 3600)
    } else {
        return format!("{} days ago", ago / (3600 * 24))
    }
}
