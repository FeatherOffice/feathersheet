//! Diagnóstico local: não grava nem imprime conteúdo da planilha.
use std::time::Instant;
fn marco(etapa: &str, inicio: Instant) {
    let memoria = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let memoria = memoria.lines().filter(|l| l.starts_with("VmRSS:") || l.starts_with("VmHWM:")).collect::<Vec<_>>().join("; ");
    eprintln!("{etapa}: {:?} {memoria}",inicio.elapsed());
}
fn main() {
    let inicio = Instant::now();
    let path = std::env::args_os().nth(1).expect("Informe um arquivo .xlsx");
    let bytes = std::fs::read(path).unwrap();
    marco("bytes",inicio);
    let json = match feathersheet_io::xlsx_read::ler_xlsx_bytes(bytes) {
        Ok(livro) => livro,
        Err(erro) => {
            marco("Importacao XLSX interrompida com seguranca", inicio);
            eprintln!("Nao foi possivel importar: {erro}");
            std::process::exit(1);
        }
    };
    marco("XML e JSON",inicio);
    let (wb,_) = feathersheet_ponte::PastaAberta::de_json(&json).extrair();
    drop(json);
    marco("pasta Rust",inicio);
    for i in 0..wb.len() {
        eprintln!("aba {i}: {} células",wb.aba(i).unwrap().sheet.len());
        for r in 0..28 { for c in 0..12 { let _ = wb.valor(i,r,c); } }
        marco("viewport calculado",inicio);
    }
}
