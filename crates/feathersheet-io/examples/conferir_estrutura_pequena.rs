//! Reproduz em tamanho pequeno a aba Dados da "Planilha Pesada" (fórmulas
//! repetidas em G–M, K com a linha 2 diferente, outra aba lendo Dados) e
//! confere, para cada operação de estrutura, se as fórmulas continuam
//! certas na memória e depois de gravar e reabrir. Mede o tempo de cada uma.
//! `conferir_estrutura_pequena [linhas]` (padrão 2000).
use feathersheet_core::planilha::workbook::Workbook;
use std::time::Instant;

fn abrir(caminho: &std::path::Path) -> Workbook {
    let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(caminho).expect("ler FSHZ");
    feathersheet_io::para_workbook::para_workbook(&lida)
}

fn gravar_e_reabrir(w: &Workbook, nome: &str) -> Workbook {
    let destino = std::env::temp_dir().join(format!("estrutura-{nome}-{}.fshz", std::process::id()));
    feathersheet_io::da_workbook::pasta_gravar_arquivo(w, 0, &destino).expect("gravar");
    let b = abrir(&destino);
    let _ = std::fs::remove_file(&destino);
    b
}

fn montar(linhas: u32) -> Workbook {
    let mut w = Workbook::new();
    w.editar(0, |s| {
        s.name = "Dados".into();
        for (c, t) in ["ID", "Data", "Região", "Produto", "Qtd", "Preço", "Total", "Desconto", "Líquido", "Mês", "Acumulado", "Categoria", "Faixa"].iter().enumerate() {
            s.set_cell(0, c as u32, t);
        }
        for r in 1..=linhas {
            let n = r + 1;
            s.set_cell(r, 0, &r.to_string());
            s.set_cell(r, 1, &(46000 + r % 300).to_string());
            s.set_cell(r, 2, ["Norte", "Sul", "Leste"][(r % 3) as usize]);
            s.set_cell(r, 3, &format!("P{:03}", r % 7));
            s.set_cell(r, 4, &(r % 100).to_string());
            s.set_cell(r, 5, &format!("{}.5", r % 50));
            s.set_cell(r, 6, &format!("=E{n}*F{n}"));
            s.set_cell(r, 7, &format!("=SE(E{n}>50;G{n}*0,1;0)"));
            s.set_cell(r, 8, &format!("=G{n}-H{n}"));
            s.set_cell(r, 9, &format!("=MÊS(B{n})"));
            s.set_cell(r, 10, &if r == 1 { format!("=I{n}") } else { format!("=K{}+I{n}", n - 1) });
            s.set_cell(r, 11, &format!("=SE(D{n}=\"P001\";\"Casa\";\"Outro\")"));
            s.set_cell(r, 12, &format!("=SE(I{n}>2000;\"Alto\";SE(I{n}>800;\"Médio\";\"Baixo\"))"));
        }
    });
    let ultima = linhas + 1;
    let i = w.criar_aba("Resumo");
    w.editar(i, |s| {
        s.set_cell(0, 0, "Norte");
        s.set_cell(0, 1, &format!("=SOMASES(Dados!$I$2:$I${ultima};Dados!$C$2:$C${ultima};A1)"));
        s.set_cell(1, 0, "Sul");
        s.set_cell(1, 1, &format!("=SOMASES(Dados!$I$2:$I${ultima};Dados!$C$2:$C${ultima};A2)"));
    });
    w
}

fn comparar(nome: &str, a: &Workbook, b: &Workbook) -> usize {
    let mut diferentes = 0;
    for i in 0..a.len().max(b.len()) {
        let (Some(sa), Some(sb)) = (a.aba(i), b.aba(i)) else { diferentes += 1; continue };
        let (sa, sb) = (&sa.sheet, &sb.sheet);
        for r in 0..=sa.max_row.max(sb.max_row) {
            for c in 0..=sa.max_col.max(sb.max_col) {
                let (x, y) = (sa.get_raw(r, c), sb.get_raw(r, c));
                if x != y {
                    diferentes += 1;
                    if diferentes <= 6 {
                        println!("   {nome} {}!({r},{c}): {x:?} → {y:?}", sa.name);
                    }
                }
            }
        }
    }
    diferentes
}

fn main() {
    let linhas: u32 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(2000);
    // Como no app: gravado e reaberto, as fórmulas viram padrões empacotados.
    let base = gravar_e_reabrir(&montar(linhas), "base");
    type Op = Box<dyn Fn(&mut Workbook)>;
    let casos: Vec<(&str, Op)> = vec![
        ("inserir coluna antes de G", Box::new(|w| { w.inserir_colunas(0, 6, 1); })),
        ("inserir coluna antes de A", Box::new(|w| { w.inserir_colunas(0, 0, 1); })),
        ("excluir coluna D", Box::new(|w| { w.excluir_colunas(0, 3, 1); })),
        ("excluir coluna H (fórmulas)", Box::new(|w| { w.excluir_colunas(0, 7, 1); })),
        ("excluir colunas G:H", Box::new(|w| { w.excluir_colunas(0, 6, 2); })),
        ("inserir 2 colunas antes de K", Box::new(|w| { w.inserir_colunas(0, 10, 2); })),
        ("inserir 2 linhas antes da 3", Box::new(|w| { w.inserir_linhas(0, 2, 2); })),
        ("excluir linha 3", Box::new(|w| { w.excluir_linhas(0, 2, 1); })),
        ("excluir linha 2", Box::new(|w| { w.excluir_linhas(0, 1, 1); })),
        ("inserir col G e excluir col M", Box::new(|w| { w.inserir_colunas(0, 6, 1); w.excluir_colunas(0, 13, 1); })),
        ("excluir col D e inserir col K", Box::new(|w| { w.excluir_colunas(0, 3, 1); w.inserir_colunas(0, 10, 1); })),
    ];
    for (nome, acao) in casos {
        let mut w = abrir_copia(&base);
        let t = Instant::now();
        acao(&mut w);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let s = &w.aba(0).unwrap().sheet;
        println!("== {nome} · {ms:.0} ms · linha 2: {}", (0..=s.max_col).map(|c| s.get_raw(1, c)).collect::<Vec<_>>().join(" | "));
        println!("   linha 3: {}", (0..=s.max_col).map(|c| s.get_raw(2, c)).collect::<Vec<_>>().join(" | "));
        if let Some(r) = w.aba(1) { println!("   Resumo B1: {}", r.sheet.get_raw(0, 1)); }
        let b = gravar_e_reabrir(&w, "op");
        let d = comparar("gravar+reabrir", &w, &b);
        if d > 0 {
            println!("   ** {d} célula(s) MUDARAM ao gravar e reabrir");
        }
    }
}

/// Cópia independente (o Workbook não é Clone barato o bastante? grava e reabre).
fn abrir_copia(w: &Workbook) -> Workbook {
    gravar_e_reabrir(w, "copia")
}
