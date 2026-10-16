use super::{selection_stats, FaixasOcultas};
use crate::celulas::refs::Rect;
use crate::planilha::sheet::Sheet;

#[test]
pub(super) fn soma_numeros_conta_texto_e_ignora_vazio_e_erro() {
    let mut s = Sheet::new("Planilha1");
    for (r, t) in ["10", "texto", "", "=1/0", "30", "VERDADEIRO"].iter().enumerate() {
        s.set_cell_raw(r as u32, 0, t.to_string(), None);
    }
    let st = selection_stats(&s, Rect::new(0, 0, 5, 0));
    assert_eq!((st.count, st.numbers), (4, 2));
    assert_eq!((st.sum, st.min, st.max), (40.0, 10.0, 30.0));
}

#[test]
pub(super) fn linha_escondida_pelo_filtro_nao_conta() {
    let mut s = Sheet::new("Planilha1");
    for r in 0..4 {
        s.set_cell_raw(r, 0, ((r + 1) * 10).to_string(), None);
    }
    s.row_heights.insert(1, 0);
    s.row_heights.insert(2, 30);
    let st = selection_stats(&s, Rect::new(0, 0, 3, 0));
    assert_eq!((st.count, st.sum), (3, 10.0 + 30.0 + 40.0));
}

#[test]
pub(super) fn coluna_inteira_so_varre_o_que_existe() {
    let mut s = Sheet::new("Planilha1");
    s.set_cell_raw(0, 0, "5".into(), None);
    let st = selection_stats(&s, Rect::new(0, 0, crate::celulas::refs::MAX_ROWS - 1, 0));
    assert_eq!((st.count, st.sum), (1, 5.0));
}
#[test]
pub(super) fn faixas_ocultas_juntam_e_respondem() {
    let f = FaixasOcultas::new(vec![(10, 12), (2, 4), (5, 5), (20, 19)]);
    assert_eq!(f, FaixasOcultas(vec![(2, 5), (10, 12)]));
    let dentro: Vec<u32> = (0..15).filter(|&r| f.contem(r)).collect();
    assert_eq!(dentro, vec![2, 3, 4, 5, 10, 11, 12]);
}
