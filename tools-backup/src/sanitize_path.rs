#![allow(dead_code)]
use std::path::{Component, Path, PathBuf};

/// given a path return a normalized version of it
pub fn sanitize_path_win32<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut result = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            // TODO: handle errors properly by retruning an option / result?
            Component::Normal(component) => {
                result.push(sanitize_component_win32(component.to_str().unwrap()))
            }
            _ => result.push(component),
        }
    }

    result
}

fn sanitize_component_win32(path: &str) -> String {
    // check that all components are valid

    // reserved characters according to the windows docs
    // https://docs.microsoft.com/en-us/windows/win32/fileio/naming-a-file
    // < 3C, > 3E, : 3A, " 22, / 2F, \ 5C, , | 7C, ? 3F, * 2A
    let mut result = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            '\u{000}'..='\u{020}' => result.push(' '),
            ':' => result.push_str("%3A"),
            '<' => result.push_str("%3C"),
            '>' => result.push_str("%3E"),
            '\"' => result.push_str("%22"),
            '/' => result.push_str("%2F"),
            '\\' => result.push_str("%5C"),
            '|' => result.push_str("%7C"),
            '?' => result.push_str("%3F"),
            '*' => result.push_str("%2A"),
            _ => result.push(c),
        }
    }
    let possible_extension = result.rfind('.');

    // TODO: replace reserved names
    // CON, PRN, AUX, NUL, COM1, COM2, COM3, COM4, COM5, COM6, COM7, COM8, COM9, LPT1, LPT2, LPT3, LPT4, LPT5, LPT6, LPT7, LPT8, and LPT9
    let (max_chars, replacement_end) = if let Some(index) = possible_extension {
        (60 - result[index..].chars().count(), index)
    } else {
        (60, result.len())
    };

    let replacement_start = result.char_indices().nth(max_chars);
    if let Some((replacement_start, _)) = replacement_start {
        result.replace_range(replacement_start..replacement_end, "");
    }

    let last_valid = result
        .char_indices()
        .rev()
        .take_while(|(_, c)| *c == '.' || *c == ' ')
        .map(|(index, _)| index)
        .min();

    // remove trailing spaces + periods
    if let Some(last_valid) = last_valid {
        if result != "." && result != ".." {
            result.truncate(last_valid);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{sanitize_component_win32, sanitize_path_win32};
    use std::path::Path;

    #[test]
    fn test_sanitize_component_win32() {
        assert_eq!(sanitize_component_win32("foo"), "foo");
        assert_eq!(sanitize_component_win32("foo:bar"), "foo%3Abar");
        assert_eq!(
            sanitize_component_win32(
                "a_very_long_path_component_is_shortened_to_be_below_60_charcters"
            ),
            "a_very_long_path_component_is_shortened_to_be_below_60_charc"
        );
        assert_eq!(
            sanitize_component_win32(
                "it also works with unicode compoents such as föö bar when the string is too long"
            ),
            "it also works with unicode compoents such as föö bar when th"
        );
        assert_eq!(
            sanitize_component_win32(
                "it_will_also_take_care_of_extensions_if_present_by_retaining_it.txt"
            ),
            "it_will_also_take_care_of_extensions_if_present_by_retai.txt"
        );
        assert_eq!(
            sanitize_component_win32("whitespace\u{000}example.txt"),
            "whitespace example.txt"
        );
        assert_eq!(
            sanitize_component_win32("whitespace\u{001}example.txt"),
            "whitespace example.txt"
        );
        assert_eq!(
            sanitize_component_win32("whitespace\texample.txt"),
            "whitespace example.txt"
        );

        assert_eq!(
            sanitize_component_win32("trailing whitespace     "),
            "trailing whitespace"
        );
        assert_eq!(
            sanitize_component_win32("trailing dots..."),
            "trailing dots"
        );
        assert_eq!(
            sanitize_component_win32("trailing mixed. . . "),
            "trailing mixed"
        );
        assert_eq!(sanitize_component_win32(".."), "..");
        assert_eq!(sanitize_component_win32("."), ".");
    }

    #[test]
    fn test_sanitize_path_win32() {
        assert_eq!(sanitize_path_win32("./foo/bar"), Path::new("./foo/bar"));
        assert_eq!(sanitize_path_win32("./foo.../bar"), Path::new("./foo/bar"));
        assert_eq!(
            sanitize_path_win32("foo\tbar/baz"),
            Path::new("foo bar/baz")
        );
        assert_eq!(
            sanitize_path_win32("/foo /bar.txt"),
            Path::new("/foo/bar.txt")
        );
    }
}
