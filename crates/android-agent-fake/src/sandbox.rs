//! 文件路径沙箱：白名单允许、黑名单默认禁止（蓝图第五节）。

/// 允许访问的路径前缀。
pub const ALLOWED_PREFIXES: &[&str] = &[
    "/sdcard/Download/",
    "/sdcard/Pictures/",
    "/data/local/tmp/hermes/",
];

/// 默认禁止的路径前缀。
pub const DENIED_PREFIXES: &[&str] = &[
    "/data/system/",
    "/data/misc/",
    "/system/",
    "/vendor/",
    "/proc/",
    "/sys/",
];

/// 路径校验结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathVerdict {
    Allowed,
    Denied,
}

/// 规范化路径：压缩重复斜杠、去除 `.` 与 `..` 段（防越权）。
pub fn normalize(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            s => segments.push(s),
        }
    }
    let mut out = String::from("/");
    out.push_str(&segments.join("/"));
    if segments.is_empty() {
        out = "/".to_string();
    } else if path.ends_with('/') && !out.ends_with('/') {
        out.push('/');
    }
    out
}

fn with_trailing_slash(path: &str) -> String {
    if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    }
}

/// 校验路径：黑名单优先，其次白名单；都不匹配则禁止（默认拒绝）。
pub fn check_path(path: &str) -> PathVerdict {
    let normalized = normalize(path);
    let probe = with_trailing_slash(&normalized);
    for denied in DENIED_PREFIXES {
        if probe.starts_with(denied) {
            return PathVerdict::Denied;
        }
    }
    for allowed in ALLOWED_PREFIXES {
        if probe.starts_with(allowed) {
            return PathVerdict::Allowed;
        }
    }
    PathVerdict::Denied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_whitelisted_paths() {
        assert_eq!(check_path("/sdcard/Download/a.txt"), PathVerdict::Allowed);
        assert_eq!(check_path("/sdcard/Pictures/"), PathVerdict::Allowed);
        assert_eq!(
            check_path("/data/local/tmp/hermes/cache/x"),
            PathVerdict::Allowed
        );
    }

    #[test]
    fn denies_blacklisted_paths() {
        assert_eq!(check_path("/system/build.prop"), PathVerdict::Denied);
        assert_eq!(check_path("/data/system/users/0.xml"), PathVerdict::Denied);
        assert_eq!(check_path("/proc/1/cmdline"), PathVerdict::Denied);
    }

    #[test]
    fn denies_unknown_paths_by_default() {
        assert_eq!(check_path("/sdcard/DCIM/x"), PathVerdict::Denied);
        assert_eq!(check_path("/"), PathVerdict::Denied);
    }

    #[test]
    fn traversal_cannot_escape_sandbox() {
        assert_eq!(
            check_path("/sdcard/Download/../../system/build.prop"),
            PathVerdict::Denied
        );
        assert_eq!(
            check_path("/sdcard/Download/../Download/ok.txt"),
            PathVerdict::Allowed
        );
    }

    #[test]
    fn normalize_collapses_segments() {
        assert_eq!(normalize("//a//b/./c/"), "/a/b/c/");
        assert_eq!(normalize("/a/b/../c"), "/a/c");
    }
}
