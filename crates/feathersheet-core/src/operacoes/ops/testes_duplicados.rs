use super::remove_duplicates;
use crate::celulas::refs::Rect;
use crate::planilha::sheet::Sheet;
use crate::celulas::values::Value;

pub(super) fn planilha(linhas: &[[&str; 2]]) -> Sheet {
    let mut s = Sheet::new("Planilha1");
    for (r, linha) in linhas.iter().enumerate() {
        for (c, texto) in linha.iter().enumerate() {
            s.set_cell_raw(r as u32, c as u32, texto.to_string(), None);
        }
    }
    s
}

pub(super) fn coluna(s: &Sheet, c: u32, ate: u32) -> Vec<String> {
    (0..=ate).map(|r| s.get_raw(r, c)).collect()
}

#[test]
pub(super) fn remove_as_repetidas_e_sobe_as_que_ficam() {
    let mut s = planilha(&[["Nome", "Cor"], ["ana", "azul"], ["bia", "rosa"], ["ana", "azul"], ["caio", "azul"]]);
    let removidas = remove_duplicates(&mut s, Rect::new(0, 0, 4, 1), true, None);
    assert_eq!(removidas, 1);
    assert_eq!(coluna(&s, 0, 4), ["Nome", "ana", "bia", "caio", ""]);
    assert_eq!(coluna(&s, 1, 4), ["Cor", "azul", "rosa", "azul", ""]);
}

#[test]
pub(super) fn compara_o_valor_mostrado_nao_o_texto_da_formula() {
    // `=A2*1` e `=A3*1` têm textos diferentes e o mesmo valor: são a mesma
    // linha para quem olha, como no TypeScript.
    let mut s = planilha(&[["5", "=A1*1"], ["5", "=A2*1"]]);
    assert_eq!(s.get_value(1, 1), Value::Number(5.0));
    assert_eq!(remove_duplicates(&mut s, Rect::new(0, 0, 1, 1), false, None), 1);
    assert_eq!(s.get_raw(1, 0), "");
}

#[test]
pub(super) fn coluna_inteira_para_na_ultima_linha_usada() {
    let mut s = planilha(&[["x", "1"], ["x", "1"]]);
    let inteira = Rect::new(0, 0, crate::celulas::refs::MAX_ROWS - 1, 1);
    assert_eq!(remove_duplicates(&mut s, inteira, false, None), 1);
    // Apagar não encolhe a área usada; o que importa é não ter escrito
    // nada além das duas linhas que existiam.
    assert!(s.used_rect().r1 <= 1, "escreveu até a linha {}", s.used_rect().r1);
    assert_eq!(s.get_raw(1, 0), "");
}

#[test]
pub(super) fn a_linha_que_sobe_leva_o_estilo() {
    let mut s = planilha(&[["a", "1"], ["a", "1"], ["b", "2"]]);
    let negrito = s
        .registrar_estilo(crate::celulas::style::Style { bold: true, ..Default::default() })
        .unwrap();
    s.set_style_idx(2, 0, negrito);
    assert_eq!(remove_duplicates(&mut s, Rect::new(0, 0, 2, 1), false, None), 1);
    assert_eq!(s.get_raw(1, 0), "b");
    assert_eq!(s.get_style_idx(1, 0), Some(negrito), "o negrito de \"b\" ficou para trás");
}

#[test]
pub(super) fn sem_repetida_nao_mexe_em_nada() {
    let mut s = planilha(&[["a", "1"], ["b", "2"]]);
    assert_eq!(remove_duplicates(&mut s, Rect::new(0, 0, 1, 1), false, None), 0);
    assert_eq!(coluna(&s, 0, 1), ["a", "b"]);
}
