//! Ordenar move as linhas direto no armazenamento (`permutar_linhas`).
use super::*;

pub(super) fn nums(s: &Sheet, c: u32, ate: u32) -> Vec<Value> {
    (0..=ate).map(|r| s.get_value(r, c)).collect()
}

pub(super) fn n(v: f64) -> Value {
    Value::Number(v)
}

#[test]
pub(super) fn formula_por_padrao_anda_com_a_linha_e_desfaz_e_refaz() {
    let mut s = Sheet::new("A");
    for (r, v) in [3.0, 1.0, 2.0].iter().enumerate() {
        s.set_cell(r as u32, 0, &v.to_string());
        s.set_cell(r as u32, 1, &format!("=A{}*10", r + 1));
    }
    assert_eq!(nums(&s, 1, 2), [n(30.0), n(10.0), n(20.0)]);
    s.clear_history();
    sort_range(&mut s, Rect::new(0, 0, 2, 1), &[SortKey { col: 0, ascending: true }], false);
    assert_eq!(nums(&s, 0, 2), [n(1.0), n(2.0), n(3.0)]);
    assert_eq!(nums(&s, 1, 2), [n(10.0), n(20.0), n(30.0)]);
    assert_eq!(s.get_raw(0, 1), "=A1*10");
    assert!(s.undo());
    assert_eq!(nums(&s, 0, 2), [n(3.0), n(1.0), n(2.0)]);
    assert_eq!(nums(&s, 1, 2), [n(30.0), n(10.0), n(20.0)]);
    assert!(s.redo());
    assert_eq!(nums(&s, 1, 2), [n(10.0), n(20.0), n(30.0)]);
}

/// Bug de 23/09/2026: ordenar os 2 milhões de linhas do usuário travou o
/// app (69 s, 5,5 GB) — cada célula regravada guardava a foto de antes e
/// de depois no desfazer. Agora a ordenação é um passo só.
#[test]
pub(super) fn ordenar_e_um_passo_so_no_desfazer() {
    let mut s = Sheet::new("A");
    for r in 0..1000u32 {
        s.set_cell(r, 0, &((r * 7919) % 1000).to_string());
        s.set_cell(r, 1, &format!("=A{}*2", r + 1));
    }
    s.clear_history();
    sort_range(&mut s, Rect::new(0, 0, 999, 1), &[SortKey { col: 0, ascending: true }], false);
    assert_eq!(s.celulas_do_proximo(false), [[0, 0, 999, 1]]);
    assert_eq!(s.get_value(999, 1), n(1998.0));
}

#[test]
pub(super) fn referencia_que_sai_da_planilha_vira_ref() {
    // Saldo corrido: B2 = B1 + A2. Levada para a linha 1, leria a linha 0.
    let mut s = Sheet::new("A");
    s.set_cell(0, 0, "5");
    s.set_cell(0, 1, "0");
    s.set_cell(1, 0, "1");
    s.set_cell(1, 1, "=B1+A2");
    sort_range(&mut s, Rect::new(0, 0, 1, 1), &[SortKey { col: 0, ascending: true }], false);
    assert_eq!(s.get_raw(0, 0), "1");
    assert!(s.get_raw(0, 1).contains("#REF!"), "ficou {:?}", s.get_raw(0, 1));
}

#[test]
pub(super) fn formula_com_intervalo_e_formula_de_fora_acompanham() {
    let mut s = Sheet::new("A");
    for (r, v) in [3.0, 1.0, 2.0].iter().enumerate() {
        s.set_cell(r as u32, 0, &v.to_string());
    }
    // Com intervalo: mora no `Map`. Na linha 3, soma A1:A3.
    s.set_cell(2, 1, "=SOMA(A$1:A3)");
    // Fora da área ordenada, lendo a primeira célula dela.
    s.set_cell(0, 4, "=A1");
    assert_eq!(s.get_value(0, 4), n(3.0));
    sort_range(&mut s, Rect::new(0, 0, 2, 1), &[SortKey { col: 0, ascending: true }], false);
    assert_eq!(s.get_value(0, 4), n(1.0), "quem lia A1 de fora recalcula");
    // A linha de "2" (onde a soma estava) foi para a linha 2: A$1:A2.
    assert_eq!(s.get_raw(1, 1), "=SOMA(A$1:A2)");
    assert_eq!(s.get_value(1, 1), n(3.0));
    s.undo();
    assert_eq!(s.get_raw(2, 1), "=SOMA(A$1:A3)");
    assert_eq!(s.get_value(0, 4), n(3.0));
}
