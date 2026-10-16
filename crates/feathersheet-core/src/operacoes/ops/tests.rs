#[test]
pub(super) fn ordenar_em_dois_niveis_e_diferenciar_maiusculas() {
    // Cidade (A→Z), depois valor (maior primeiro); "abc" antes de "ABC".
    let dados = [["Rio", "1"], ["Belém", "5"], ["Rio", "9"], ["ABC", "0"], ["abc", "0"]];
    let valor = |r: u32, c: u32| {
        let t = dados[r as usize][c as usize];
        t.parse::<f64>().map(Value::Number).unwrap_or_else(|_| Value::Text(t.into()))
    };
    let chaves = [SortKey { col: 0, ascending: true }, SortKey { col: 1, ascending: false }];
    let plano = planejar_ordenacao_com(Rect::new(0, 0, 4, 1), &chaves, false, 4, true, valor).unwrap();
    assert_eq!(plano.ordem, vec![4, 3, 1, 2, 0]);
    // Sem a opção, "ABC" e "abc" empatam e ficam na ordem de origem.
    let plano = planejar_ordenacao(Rect::new(0, 0, 4, 1), &chaves, false, 4, valor).unwrap();
    assert_eq!(plano.ordem, vec![3, 4, 1, 2, 0]);
}

use super::*;

pub(super) fn opcoes(match_case: bool, whole_cell: bool, in_formulas: bool) -> SearchOptions {
    SearchOptions { match_case, whole_cell, in_formulas }
}

#[test]
pub(super) fn test_find_all_basic() {
    let mut sheet = Sheet::new("Teste");
    sheet.set_cell(0, 0, "Venda de Maçã");
    sheet.set_cell(1, 0, "Venda de Banana");
    sheet.set_cell(2, 0, "Maçã Verde");
    let matches = find_all(&sheet, "maçã", opcoes(false, false, false));
    assert_eq!(matches, vec![SearchMatch { row: 0, col: 0 }, SearchMatch { row: 2, col: 0 }]);
}

#[test]
pub(super) fn test_find_all_case_sensitive() {
    let mut sheet = Sheet::new("Teste");
    sheet.set_cell(0, 0, "EXCEL");
    sheet.set_cell(1, 0, "excel");
    let matches = find_all(&sheet, "EXCEL", opcoes(true, false, false));
    assert_eq!(matches, vec![SearchMatch { row: 0, col: 0 }]);
}

/// A coluna de fórmulas empacotadas (uma fórmula preenchida para baixo
/// é um padrão só) não era olhada: a busca não achava nenhuma fórmula
/// dela, nem pelo texto nem pelo resultado.
#[test]
pub(super) fn acha_formula_da_coluna_empacotada_pelo_texto_e_pelo_valor() {
    let mut s = Sheet::new("A");
    let linhas: Vec<Vec<String>> = (1..=3).map(|i| vec![i.to_string(), format!("=A{i}*10")]).collect();
    s.paste(0, 0, &linhas, false);
    assert!(s.formulas.size() > 0, "o teste precisa da coluna empacotada");
    assert_eq!(find_all(&s, "=A2", opcoes(false, false, true)), vec![SearchMatch { row: 1, col: 1 }]);
    assert_eq!(find_all(&s, "30", opcoes(false, true, false)), vec![SearchMatch { row: 2, col: 1 }]);
}

/// O número aparece como o JavaScript escreveria (`String(n)`), não como
/// o `to_string` do Rust.
#[test]
pub(super) fn numero_e_procurado_como_aparece() {
    let mut s = Sheet::new("A");
    s.set_cell(0, 0, "1e21");
    s.set_cell(1, 0, "0,5");
    assert_eq!(find_all(&s, "1e+21", opcoes(false, true, false)).len(), 1);
    assert_eq!(find_all(&s, "0.5", opcoes(false, true, false)).len(), 1);
}

#[test]
pub(super) fn substituir_valor_calculado_sem_texto_correspondente_nao_marca_edicao() {
    let mut sheet = Sheet::new("Teste");
    sheet.set_cell(0, 0, "12");
    sheet.set_cell(1, 0, "=A1*2");
    // Busca por valor exibido encontra 24, mas a fórmula não inclui "24".
    let matches = find_all(&sheet, "24", opcoes(false, false, false));
    assert_eq!(matches, vec![SearchMatch { row: 1, col: 0 }]);
    assert_eq!(replace_all_em(&mut sheet, &matches, "24", "90",
        opcoes(false, false, false)), 0);
    assert_eq!(sheet.get_raw(1, 0), "=A1*2");
}

#[test]
pub(super) fn test_replace_all() {
    let mut sheet = Sheet::new("Teste");
    sheet.set_cell(0, 0, "São Paulo");
    sheet.set_cell(1, 0, "Rio de Janeiro");
    sheet.set_cell(2, 0, "São José");
    let count = replace_all(&mut sheet, "são", "Santo", opcoes(false, false, false));
    assert_eq!(count, 2);
    assert_eq!(sheet.get_raw(0, 0), "Santo Paulo");
    assert_eq!(sheet.get_raw(2, 0), "Santo José");
}

/// Substituir era gravado por fora do desfazer e sem recalcular quem lê.
#[test]
pub(super) fn substituir_desfaz_num_ctrl_z_so_e_recalcula() {
    let mut s = Sheet::new("A");
    s.set_cell(0, 0, "2");
    s.set_cell(1, 0, "2");
    s.set_cell(2, 0, "=SOMA(A1:A2)");
    assert_eq!(s.get_value(2, 0), Value::Number(4.0));
    assert_eq!(replace_all(&mut s, "2", "5", opcoes(false, true, false)), 2);
    assert_eq!(s.get_value(2, 0), Value::Number(10.0), "quem lê as células trocadas recalcula");
    assert!(s.undo());
    assert_eq!(s.get_raw(0, 0), "2");
    assert_eq!(s.get_raw(1, 0), "2");
    assert_eq!(s.get_value(2, 0), Value::Number(4.0));
}

#[test]
pub(super) fn trocar_sem_diferenciar_maiusculas_nao_quebra_letra_que_muda_de_tamanho() {
    assert_eq!(replace_every("Aİb İB", "b", "x", false), "Aİx İx");
    assert_eq!(replace_every("İstanbul", "i̇s", "Is", false), "Istanbul");
    assert_eq!(replace_every("abcABC", "bc", "-", false), "a-A-");
}

#[test]
pub(super) fn estatisticas_param_na_ultima_linha_usada() {
    let mut s = Sheet::new("A");
    s.set_cell(0, 0, "3");
    s.set_cell(1, 0, "texto");
    s.set_cell(2, 0, "5");
    s.set_cell(3, 0, "3");
    let st = column_stats(&s, Rect::new(0, 0, MAX_ROWS - 1, 0));
    assert_eq!((st.count, st.numbers, st.empty, st.distinct), (4, 3, 0, 3));
    assert_eq!((st.sum, st.min, st.max), (11.0, 3.0, 5.0));
}

#[test]
pub(super) fn autosoma_prefere_o_bloco_de_cima() {
    let mut s = Sheet::new("A");
    for (r, v) in [(0, "1"), (1, "2"), (2, "3")] {
        s.set_cell(r, 3, v);
    }
    s.set_cell(5, 0, "4");
    s.set_cell(5, 1, "6");
    let ler = |r: u32, c: u32| s.get_value(r, c);
    assert_eq!(formula_da_autosoma(3, 3, ler).as_deref(), Some("=SOMA(D1:D3)"));
    assert_eq!(formula_da_autosoma(5, 2, ler).as_deref(), Some("=SOMA(A6:B6)"));
    assert_eq!(formula_da_autosoma(9, 9, ler), None);
}
