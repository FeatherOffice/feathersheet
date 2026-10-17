//! Fórmula que lê intervalo local na coluna empacotada (17/10/2026): um
//! padrão para a coluna toda, sem uma dependência por célula no grafo.
//! O caso do usuário: `=MÍNIMOSES(AO:AO;A:A;A2;C:C;C2;AP:AP;AP2)` em 13,7
//! milhões de linhas. Aqui, com poucas linhas, confere o que não pode mudar:
//! o resultado, quem recalcula quando um dado muda, edição estrutural e
//! desfazer.
use feathersheet_core::celulas::refs::Rect;
use feathersheet_core::planilha::sheet::Sheet;
use feathersheet_core::planilha::workbook::Workbook;
use feathersheet_core::Value;

fn num(v: Value) -> f64 {
    match v {
        Value::Number(n) => n,
        outro => panic!("esperava número, veio {outro:?}"),
    }
}

/// A: grupo, B: valor. Grupos a (linhas 2–4) e b (5–6).
fn dados(s: &mut Sheet) {
    for (r, (g, v)) in [("a", 5), ("a", 3), ("a", 9), ("b", 7), ("b", 2)].into_iter().enumerate() {
        s.set_cell(r as u32 + 1, 0, g);
        s.set_cell(r as u32 + 1, 1, &v.to_string());
    }
}

#[test]
fn minimoses_em_coluna_inteira_fica_empacotado_e_recalcula() {
    let mut s = Sheet::new("Dados");
    dados(&mut s);
    s.set_cell(1, 3, "=MÍNIMOSES(B:B;A:A;A2)");
    s.preencher(&Rect::new(1, 3, 1, 3), &Rect::new(1, 3, 5, 3));
    assert!(s.formulas.has(4, 3), "D5 mora na coluna empacotada");
    assert!(!s.cells.contains_key(&feathersheet_core::celulas::refs::key(4, 3)), "e não no mapa de avulsas");
    assert_eq!(s.get_raw(4, 3), "=MÍNIMOSES(B:B;A:A;A5)");
    let todos = |s: &Sheet| (1..=5).map(|r| num(s.get_value(r, 3))).collect::<Vec<_>>();
    assert_eq!(todos(&s), [3., 3., 3., 2., 2.]);

    // Um valor dentro de B:B muda: o grupo a recalcula.
    s.set_cell(3, 1, "1");
    assert_eq!(todos(&s), [1., 1., 1., 2., 2.]);
    // Um critério muda: a linha 4 passa para o grupo b.
    s.set_cell(3, 0, "b");
    assert_eq!(todos(&s), [3., 3., 1., 1., 1.]);
    // Uma linha nova lá embaixo, ainda dentro de A:A e B:B.
    s.set_cell(500, 0, "a");
    s.set_cell(500, 1, "-4");
    assert_eq!(todos(&s), [-4., -4., 1., 1., 1.]);
}

#[test]
fn janela_relativa_recalcula_so_quem_le_a_celula() {
    let mut s = Sheet::new("A");
    for r in 0..10 {
        s.set_cell(r, 0, &(r + 1).to_string());
    }
    // C3 = SOMA(A1:A3), descendo: janela de três linhas.
    s.set_cell(2, 2, "=SOMA(A1:A3)");
    s.preencher(&Rect::new(2, 2, 2, 2), &Rect::new(2, 2, 9, 2));
    assert!(s.formulas.has(9, 2));
    assert_eq!(num(s.get_value(9, 2)), 8. + 9. + 10.);
    s.set_cell(4, 0, "100"); // A5
    // Leem A5: C5, C6 e C7.
    assert_eq!(num(s.get_value(4, 2)), 3. + 4. + 100.);
    assert_eq!(num(s.get_value(6, 2)), 100. + 6. + 7.);
    assert_eq!(num(s.get_value(7, 2)), 6. + 7. + 8.);
    assert_eq!(num(s.get_value(3, 2)), 2. + 3. + 4.);
}

#[test]
fn intervalo_na_propria_coluna_continua_avulso() {
    let mut s = Sheet::new("A");
    s.set_cell(0, 0, "1");
    s.set_cell(5, 0, "=SOMA(A1:A3)");
    assert!(!s.formulas.has(5, 0), "leria a própria coluna");
    assert_eq!(num(s.get_value(5, 0)), 1.);
}

#[test]
fn inserir_coluna_antes_do_intervalo_conserta_o_texto() {
    let mut w = Workbook::new();
    {
        let s = &mut w.aba_mut(0).unwrap().sheet;
        dados(s);
        s.set_cell(1, 3, "=MÍNIMOSES(B:B;A:A;A2)");
        s.preencher(&Rect::new(1, 3, 1, 3), &Rect::new(1, 3, 5, 3));
    }
    w.inserir_colunas(0, 1, 1); // nova coluna B
    let s = &w.aba(0).unwrap().sheet;
    assert_eq!(s.get_raw(4, 4), "=MÍNIMOSES(C:C;A:A;A5)");
    assert_eq!(num(s.get_value(4, 4)), 2.);
    assert_eq!(num(s.get_value(1, 4)), 3.);
}

#[test]
fn desfazer_o_preenchimento() {
    let mut s = Sheet::new("Dados");
    dados(&mut s);
    s.set_cell(1, 3, "=MÍNIMOSES(B:B;A:A;A2)");
    s.preencher(&Rect::new(1, 3, 1, 3), &Rect::new(1, 3, 5, 3));
    assert!(s.undo());
    assert_eq!(s.get_raw(4, 3), "");
    assert_eq!(s.get_raw(1, 3), "=MÍNIMOSES(B:B;A:A;A2)");
    assert_eq!(num(s.get_value(1, 3)), 3.);
}

#[test]
fn formula_avulsa_com_coluna_inteira_anda_ao_inserir_coluna() {
    let mut w = Workbook::new();
    w.escrever_celula(0, 1, 3, "=SOMA(B:B)+INDIRETO(\"A1\")".into());
    w.inserir_colunas(0, 1, 1);
    assert_eq!(w.aba(0).unwrap().sheet.get_raw(1, 4), "=SOMA(C:C)+INDIRETO(\"A1\")");
}

#[test]
fn preencher_formula_em_bloco_grava_um_padrao_e_desfaz_num_passo() {
    let mut s = Sheet::new("Dados");
    let n = 300_000u32;
    for r in 1..=n {
        s.set_cell(r, 0, if r % 2 == 0 { "par" } else { "ímpar" });
        s.set_cell(r, 1, &r.to_string());
    }
    s.set_cell(1, 3, "=MÍNIMOSES(B:B;A:A;A2)");
    assert!(s.faixa_vazia(3, 2, n));
    let t = std::time::Instant::now();
    assert!(s.preencher_formula_faixa_vazia(3, 2, n, "=MÍNIMOSES(B:B;A:A;A3)"));
    assert!(t.elapsed() < std::time::Duration::from_millis(500), "{:?}", t.elapsed());
    assert_eq!(s.get_raw(n, 3), format!("=MÍNIMOSES(B:B;A:A;A{})", n + 1));
    assert_eq!(num(s.get_value(n, 3)), 2.);
    assert_eq!(num(s.get_value(n - 1, 3)), 1.);
    // Um dado novo no grupo par.
    s.set_cell(n + 5, 0, "par");
    s.set_cell(n + 5, 1, "-1");
    assert_eq!(num(s.get_value(n, 3)), -1.);
    assert!(s.undo(), "desfaz o dado");
    assert!(s.undo(), "desfaz o dado");
    assert!(s.undo(), "desfaz o preenchimento num passo só");
    assert_eq!(s.get_raw(n, 3), "");
    assert_eq!(s.get_raw(1, 3), "=MÍNIMOSES(B:B;A:A;A2)");
    assert!(s.redo());
    assert_eq!(num(s.get_value(n, 3)), 2.);
    // Volátil, ou lendo a própria coluna, não vai por aqui.
    assert!(!s.preencher_formula_faixa_vazia(4, 2, 10, "=AGORA()"));
    assert!(!s.preencher_formula_faixa_vazia(4, 2, 10, "=SOMA(E:E)"));
}

#[test]
fn soltar_faixas_nao_joga_fora_o_indice_dos_ses() {
    let mut s = Sheet::new("Dados");
    for r in 0..5000u32 {
        s.set_cell(r, 0, if r % 2 == 0 { "a" } else { "b" });
        s.set_cell(r, 1, &r.to_string());
    }
    let soma = Rect::new(0, 1, 4999, 1);
    let criterio = [Rect::new(0, 0, 4999, 0)];
    let antes = s.agregado(Some(&soma), &criterio, None).expect("índice montado");
    s.soltar_faixas(); // o pulso do monitor, a cada segundo
    let depois = s.agregado(Some(&soma), &criterio, None).unwrap();
    assert!(std::sync::Arc::ptr_eq(&antes, &depois), "o índice tem de ser o mesmo, sem remontar");
    s.set_cell(3, 1, "-7");
    let novo = s.agregado(Some(&soma), &criterio, None).unwrap();
    assert!(!std::sync::Arc::ptr_eq(&antes, &novo), "a aba mudou: aí sim remonta");
}
