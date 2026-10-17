//! Abre um `.fshz`, grava de novo e reabre: as fórmulas e os valores têm de
//! ser os mesmos, célula por célula. `conferir_ida_e_volta ARQUIVO.fshz [linhas]`.
fn abrir(caminho: &std::path::Path) -> feathersheet_core::planilha::workbook::Workbook {
    let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(caminho).expect("ler FSHZ");
    feathersheet_io::para_workbook::para_workbook(&lida)
}

fn main() {
    let caminho = std::path::PathBuf::from(std::env::args().nth(1).expect("arquivo"));
    let linhas: u32 = std::env::args().nth(2).and_then(|n| n.parse().ok()).unwrap_or(5_000);
    let a = abrir(&caminho);
    let destino = std::env::temp_dir().join(format!("ida-e-volta-{}.fshz", std::process::id()));
    feathersheet_io::da_workbook::pasta_gravar_arquivo(&a, 0, &destino).expect("gravar");
    let b = abrir(&destino);
    let _ = std::fs::remove_file(&destino);
    let mut diferentes = 0;
    for i in 0..a.len() {
        let (sa, sb) = (&a.aba(i).unwrap().sheet, &b.aba(i).unwrap().sheet);
        for r in 0..=sa.max_row.min(linhas) {
            for c in 0..=sa.max_col {
                let (x, y) = (sa.get_raw(r, c), sb.get_raw(r, c));
                if x != y {
                    diferentes += 1;
                    if diferentes <= 15 {
                        println!("{}!({r},{c}): antes {x:?} · depois {y:?}", sa.name);
                    }
                }
            }
        }
    }
    println!("{diferentes} célula(s) diferentes nas primeiras {linhas} linhas de cada aba");
}
