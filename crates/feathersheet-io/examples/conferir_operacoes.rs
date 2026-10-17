//! Aplica operações de estrutura na aba Dados de um `.fshz` e confere se as
//! fórmulas continuam coerentes (linha 2 e linha 3 com o mesmo padrão, e
//! referências deslocadas como deveriam). `conferir_operacoes ARQUIVO.fshz`.
fn abrir(caminho: &std::path::Path) -> feathersheet_core::planilha::workbook::Workbook {
    let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(caminho).expect("ler FSHZ");
    feathersheet_io::para_workbook::para_workbook(&lida)
}

fn linhas(w: &feathersheet_core::planilha::workbook::Workbook, aba: usize, r: u32) -> String {
    let s = &w.aba(aba).unwrap().sheet;
    (0..=s.max_col).map(|c| s.get_raw(r, c)).collect::<Vec<_>>().join(" | ")
}

fn main() {
    let caminho = std::path::PathBuf::from(std::env::args().nth(1).expect("arquivo"));
    let dados = abrir(&caminho).indice_por_nome("Dados").expect("aba Dados");
    let casos: Vec<(&str, Box<dyn Fn(&mut feathersheet_core::planilha::workbook::Workbook)>)> = vec![
        ("original", Box::new(|_| {})),
        ("inserir coluna antes de G", Box::new(move |w| { w.inserir_colunas(dados, 6, 1); })),
        ("excluir coluna D", Box::new(move |w| { w.excluir_colunas(dados, 3, 1); })),
        ("inserir 2 linhas antes da 3", Box::new(move |w| { w.inserir_linhas(dados, 2, 2); })),
        ("excluir linha 3", Box::new(move |w| { w.excluir_linhas(dados, 2, 1); })),
    ];
    for (nome, acao) in casos {
        let mut w = abrir(&caminho);
        acao(&mut w);
        println!("== {nome}");
        for r in [1u32, 2, 3, 999_999] {
            println!("  linha {}: {}", r + 1, linhas(&w, dados, r));
        }
        // E depois de gravar e reabrir.
        let destino = std::env::temp_dir().join(format!("conferir-op-{}.fshz", std::process::id()));
        feathersheet_io::da_workbook::pasta_gravar_arquivo(&w, 0, &destino).expect("gravar");
        let b = abrir(&destino);
        let _ = std::fs::remove_file(&destino);
        for r in [1u32, 2, 3] {
            if linhas(&w, dados, r) != linhas(&b, dados, r) {
                println!("  ** MUDOU AO GRAVAR E REABRIR, linha {}: {}", r + 1, linhas(&b, dados, r));
            }
        }
    }
}
