use std::path::PathBuf;

#[allow(dead_code)]
pub fn assets() -> PathBuf {
    std::env::var("CRIFS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"F:\Codes\CriFsV2Lib\CriFsV2Lib.Tests\Assets"))
}

#[allow(dead_code)]
pub fn asset(name: &str) -> PathBuf {
    assets().join(name)
}

#[allow(dead_code)]
pub fn corpora() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(extra) = std::env::var("CPK_CORPUS") {
        roots.push(PathBuf::from(extra));
    }
    roots.push(PathBuf::from(
        r"F:\Codes\UNICORN OVERLORD\extract\160d1db236ac9d15b509ef661bf18b7b_NCA\1\Unicorn.CPK",
    ));
    roots.push(PathBuf::from(r"F:\Codes\13\main\1\ROBO_CN.CPK"));
    roots.push(PathBuf::from(r"F:\Codes\13\dlc\1\ROBO_DLC_CN.CPK"));
    roots.push(PathBuf::from(
        r"F:\Codes\UNICORN OVERLORD\extract\art book\ca8a50e7569e2c38f6879a4e8f57cd42_NCA\1\UnicornDLC.CPK",
    ));
    roots.retain(|path| path.is_file());
    roots
}
