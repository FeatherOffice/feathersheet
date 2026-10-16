//! Diagnóstico do leitor XLSX nativo (não imprime conteúdo das células).
//! Acrescente --calcular para avaliar também as células após a importação.
//! cargo run --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml \
//!   --example diagnostico_xlsx --release -j 2 -- /caminho/arquivo.xlsx
use std::env;
use std::fs;
use std::time::Instant;

#[cfg(target_os = "linux")]
fn memoria_mib(chave: &str) -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let linha = status.lines().find(|l| l.starts_with(chave))?;
    linha.split_whitespace().nth(1)?.parse::<u64>().ok().map(|kb| kb / 1024)
}

#[cfg(not(target_os = "linux"))]
fn memoria_mib(_: &str) -> Option<u64> { None }

fn main() {
    let path = match env::args_os().nth(1) {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            eprintln!("Uso: diagnostico_xlsx /caminho/arquivo.xlsx");
            std::process::exit(2);
        }
    };
    let inicio = Instant::now();
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Erro lendo o arquivo: {e}");
            std::process::exit(1);
        }
    };
    println!("Arquivo: {} bytes compactados", bytes.len());
    let (book, ativa) = match feathersheet_io::xlsx_read::ler_xlsx_workbook(bytes) {
        Ok(book) => book,
        Err(e) => {
            eprintln!("Falha na importação XLSX: {e}");
            eprintln!("Tempo até erro: {:.2}s", inicio.elapsed().as_secs_f64());
            eprintln!("Pico de RAM: {:?} MiB", memoria_mib("VmHWM:"));
            std::process::exit(1);
        }
    };
    let mut ocupadas = 0u64;
    for i in 0..book.len() {
        if let Some(aba) = book.aba(i) {
            let sheet = &aba.sheet;
            let total = sheet.data.size() + sheet.cells.len() as u64 + u64::from(sheet.formulas.size());
            ocupadas += total;
            println!("Aba {}: {} (aprox. {} células com conteúdo)", i + 1, sheet.name, total);
        }
    }
    if env::args().any(|arg| arg == "--calcular") {
        for i in 0..book.len() {
            let sheet = &book.aba(i).unwrap().sheet;
            let chaves = sheet.chaves_no_retangulo(&sheet.used_rect());
            eprintln!("Calculando aba {} ({} células)", i + 1, chaves.len());
            for chave in chaves {
                let (r, c) = feathersheet_core::planilha::sheet::Sheet::key_to_coord(chave);
                let _ = book.valor(i, r, c);
            }
        }
    }
    println!("Aba ativa: {}", ativa + 1);
    println!("Total aproximado de células com conteúdo: {ocupadas}");
    println!("Tempo: {:.2}s", inicio.elapsed().as_secs_f64());
    println!("RAM atual: {:?} MiB", memoria_mib("VmRSS:"));
    println!("Pico de RAM: {:?} MiB", memoria_mib("VmHWM:"));
}
