//! O `paste` sem desfazer por tipo de célula, 1 milhão de linhas × 4 colunas.
use feathersheet_core::planilha::sheet::Sheet;

type Gerador = fn(usize) -> String;
fn main() {
    let tipos: [(&str, Gerador); 4] = [
        ("inteiro", |i| (i % 1000).to_string()),
        ("decimal", |i| format!("0.{}", i % 997)),
        ("texto", |i| format!("Faixa {}", i % 50_000)),
        ("booleano", |i| if i % 2 == 0 { "True".into() } else { "False".into() }),
    ];
    for (nome, gerar) in tipos {
        let linhas: Vec<Vec<String>> = (0..1_000_000).map(|i| (0..4).map(|_| gerar(i)).collect()).collect();
        let mut s = Sheet::new("A");
        let t = std::time::Instant::now();
        for (k, lote) in linhas.chunks(50_000).enumerate() {
            s.paste((k * 50_000) as u32, 0, lote, false);
        }
        eprintln!("{nome:9}: {} ns/célula", t.elapsed().as_nanos() / 4_000_000);
    }
}
