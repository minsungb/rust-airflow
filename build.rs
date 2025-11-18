use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Cargo 빌드 단계에서 아이콘과 폰트 리소스를 준비한다.
fn main() {
    embed_icon();
    generate_font_stub().expect("폰트 스텁 생성 실패");
}

#[cfg(target_os = "windows")]
/// Windows 빌드에서 icon.ico가 존재하면 리소스로 임베드한다.
fn embed_icon() {
    let icon_path = Path::new("icons/icon.ico");
    if !icon_path.exists() {
        println!("cargo:warning=icons/icon.ico 파일이 없어 기본 아이콘이 사용됩니다.");
        return;
    }
    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path.to_string_lossy().as_ref());
    res.compile().expect("아이콘 리소스 컴파일 실패");
}

#[cfg(not(target_os = "windows"))]
/// 비 Windows 환경에서는 파일 변경 감지만 수행한다.
fn embed_icon() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
}

/// 사용 가능한 한글 폰트를 탐색하여 egui에서 include_bytes! 할 수 있는 스텁을 만든다.
fn generate_font_stub() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("custom_font.rs");
    if let Some(path) = locate_font_file() {
        let escaped = path.to_string_lossy().replace('\\', "\\\\");
        let content = format!(
            "pub fn embedded_font_bytes() -> Option<&'static [u8]> {{\n    Some(include_bytes!(\"{escaped}\"))\n}}\n"
        );
        fs::write(dest, content)?;
    } else {
        fs::write(
            dest,
            "pub fn embedded_font_bytes() -> Option<&'static [u8]> { None }\n",
        )?;
    }
    Ok(())
}

/// 시스템 폰트 디렉터리에서 한글 폰트를 찾는다.
fn locate_font_file() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
    ];
    if let Some(home) = env::var_os("HOME") {
        candidates.push(Path::new(&home).join(".fonts"));
    }
    if let Some(windir) = env::var_os("WINDIR") {
        candidates.push(Path::new(&windir).join("Fonts"));
    }
    let keywords = ["noto", "nanum", "malgun", "applegothic", "pretendard"];
    for dir in candidates {
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.into_path();
            let lower = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if !keywords.iter().any(|k| lower.contains(k)) {
                continue;
            }
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc") {
                    println!("cargo:rerun-if-changed={}", path.display());
                    return Some(path);
                }
            }
        }
    }
    println!("cargo:warning=한글 폰트를 찾지 못했습니다. 시스템 기본 폰트가 사용됩니다.");
    None
}
