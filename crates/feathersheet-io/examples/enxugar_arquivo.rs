//! Roda o enxugador num `.fshz` de verdade e mostra o que ele economizou.
//!
//! `cargo run --release --example enxugar_arquivo -- /caminho/arquivo.fshz`
//!
//! Existe porque a lição de 21/09 foi justamente esta: número de teste
//! sintético não prova nada sobre o arquivo da pessoa.

/// Pico de memória residente desde o início do processo (`VmHWM`).
///
/// É ESTE o número que importa, e ler `VmRSS` no fim foi o erro da primeira
/// medição deste exemplo: o `VmRSS` depois da função já não inclui os buffers
/// que morreram dentro dela, e foi por isso que 681 MB aqui viraram 12 GB no
/// aplicativo.
fn pico_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("VmHWM:"))
                .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<u64>().ok())
        })
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

fn rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("VmRSS:"))
                .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<u64>().ok())
        })
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

fn main() {
    let caminho = std::env::args().nth(1).expect("passe o caminho do .fshz");
    let bytes = std::fs::read(&caminho).expect("não consegui ler");
    println!("arquivo            {:>10} MB", bytes.len() / 1_048_576);
    println!("RSS ao ler         {:>10} MB", rss_mb());

    let t = std::time::Instant::now();
    let (saida, resumo) = feathersheet_io::enxugar::ler_e_enxugar(&bytes).expect("falhou");
    let ms = t.elapsed().as_millis();

    println!("RSS no fim         {:>10} MB", rss_mb());
    println!("PICO REAL (VmHWM)  {:>10} MB", pico_mb());
    println!("tempo              {ms:>10} ms");
    println!("corpo enxuto       {:>10} MB  (vai para a WebView)", saida.len() / 1_048_576);
    println!("avulsas antes      {:>10}", resumo.avulsas_antes);
    println!("avulsas depois     {:>10}", resumo.avulsas_depois);
    println!("colunas colapsadas {:>10}", resumo.colunas_colapsadas);
    println!("vazias descartadas {:>10}", resumo.vazias_descartadas);
}
