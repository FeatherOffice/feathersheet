//! Os mesmos casos do `test/core/duplicados.test.ts`.
use super::{regiao_atual, remove_duplicates};
use crate::celulas::refs::{Rect, MAX_ROWS};
use crate::planilha::sheet::Sheet;

pub(super) fn planilha() -> Sheet {
    let mut s = Sheet::new("Planilha1");
    let linhas = [
        ["ID", "Nome", "Cidade", "Status", "Obs"],
        ["1", "ana", "Rio", "ALTO", "o1"],
        ["2", "bia", "SP", "ALTO", "o2"],
        ["3", "caio", "BH", "NORMAL", "o3"],
        ["4", "davi", "Rio", "NORMAL", "o4"],
    ];
    for (r, linha) in linhas.iter().enumerate() {
        for (c, v) in linha.iter().enumerate() {
            s.set_cell_raw(r as u32, c as u32, v.to_string(), None);
        }
    }
    s.set_cell_raw(0, 6, "longe".into(), None);
    s
}

pub(super) fn linha(s: &Sheet, r: u32) -> Vec<String> {
    (0..5).map(|c| s.get_raw(r, c)).collect()
}

#[test]
pub(super) fn para_na_coluna_vazia_e_deixa_a_g_de_fora() {
    let s = planilha();
    assert_eq!(regiao_atual(&s, Rect::new(2, 3, 2, 3)), Rect::new(0, 0, 4, 4));
    assert_eq!(regiao_atual(&s, Rect::new(0, 3, MAX_ROWS - 1, 3)), Rect::new(0, 0, 4, 4));
}

#[test]
pub(super) fn conta_a_diagonal_e_celula_isolada_e_ela_mesma() {
    let mut s = Sheet::new("Planilha1");
    s.set_cell_raw(0, 0, "a".into(), None);
    s.set_cell_raw(1, 1, "b".into(), None);
    s.set_cell_raw(5, 5, "c".into(), None);
    assert_eq!(regiao_atual(&s, Rect::new(0, 0, 0, 0)), Rect::new(0, 0, 1, 1));
    assert_eq!(regiao_atual(&s, Rect::new(5, 5, 5, 5)), Rect::new(5, 5, 5, 5));
}

#[test]
pub(super) fn comparando_so_a_d_a_linha_inteira_sobe_junto() {
    let mut s = planilha();
    assert_eq!(remove_duplicates(&mut s, Rect::new(0, 0, 4, 4), true, Some(&[3])), 2);
    assert_eq!(linha(&s, 1), ["1", "ana", "Rio", "ALTO", "o1"]);
    assert_eq!(linha(&s, 2), ["3", "caio", "BH", "NORMAL", "o3"]);
    assert_eq!(linha(&s, 3), ["", "", "", "", ""]);
    assert_eq!(s.get_raw(0, 6), "longe");
}
