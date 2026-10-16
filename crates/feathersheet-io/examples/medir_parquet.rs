//! Abre um `.parquet` pelo leitor do projeto e mede tempo e memória.
//!   systemd-run --user --scope -p MemoryMax=16G cargo run --release --example medir_parquet -- arquivo.parquet
fn rss(chave: &str) -> f64 {
    std::fs::read_to_string("/proc/self/status").unwrap_or_default().lines()
        .find_map(|l| l.strip_prefix(chave))
        .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}
fn main() -> Result<(), String> {
    let caminho = std::env::args().nth(1).ok_or("informe o arquivo")?;
    let t = std::time::Instant::now();
    let wb = feathersheet_io::parquet_read::ler_parquet_arquivo(std::path::Path::new(&caminho))?;
    let s = &wb.aba(0).ok_or("sem aba")?.sheet;
    let u = s.used_rect();
    println!("{:.1} s · RAM {:.0} MB (pico {:.0}) · área usada {}x{} · A2={:?} F2={:?}",
        t.elapsed().as_secs_f64(), rss("VmRSS:"), rss("VmHWM:"), u.r1 + 1, u.c1 + 1, s.get_value(1, 0), s.get_value(1, 5));
    Ok(())
}
