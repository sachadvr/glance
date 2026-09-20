//! The theme chosen last time, kept as its name in the user's config directory.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

/// The saved name, if a previous session wrote one.
pub(super) fn load() -> Option<String> {
    let path = saved_choice_path(std::env::var_os("XDG_CONFIG_HOME"), std::env::var_os("HOME"))?;
    fs::read_to_string(path).ok()
}

pub(super) fn save(name: &str) {
    let Some(path) = saved_choice_path(std::env::var_os("XDG_CONFIG_HOME"), std::env::var_os("HOME")) else {
        return;
    };
    // Remembering the choice is a convenience: a read-only config directory must not stop the switch.
    let _ = path.parent().map(fs::create_dir_all);
    let _ = fs::write(path, name);
}

// `$XDG_CONFIG_HOME/glance/theme`, else `$HOME/.config/glance/theme`: the environment is passed in so the rule is testable.
fn saved_choice_path(xdg_config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let config = xdg_config_home.map(PathBuf::from).or_else(|| home.map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("glance").join("theme"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_choice_path_prefers_xdg_config_home() {
        let path = saved_choice_path(Some("/xdg".into()), Some("/home/me".into()));
        assert_eq!(path, Some(PathBuf::from("/xdg/glance/theme")));
    }

    #[test]
    fn saved_choice_path_falls_back_to_the_home_config_directory() {
        assert_eq!(saved_choice_path(None, Some("/home/me".into())), Some(PathBuf::from("/home/me/.config/glance/theme")));
    }

    #[test]
    fn saved_choice_path_is_none_without_either_variable() {
        assert_eq!(saved_choice_path(None, None), None);
    }
}
