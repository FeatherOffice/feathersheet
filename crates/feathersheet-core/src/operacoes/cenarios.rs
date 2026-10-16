//! Gerenciador de cenários do Excel: conjuntos de valores para as mesmas
//! células variáveis. O motor aplica um cenário e monta o resumo
//! (resultado de cada cenário nas células de resultado). Guardar e ler o
//! JSON dos cenários (`Sheet::ce`) é da interface: o motor não tem serde.
use crate::celulas::values::Value;
use crate::planilha::{sheet::edicao::Snap, workbook::Workbook};

/// O Excel limita um cenário a 32 células variáveis.
pub const MAX_CELULAS: usize = 32;

/// Um cenário já lido: nome e o conteúdo de cada célula variável.
#[derive(Debug, Clone, PartialEq)]
pub struct Cenario {
    pub nome: String,
    pub valores: Vec<((u32, u32), String)>,
}

/// "Mostrar": escreve os valores do cenário numa transação (um Ctrl+Z).
pub fn aplicar(workbook: &mut Workbook, aba: usize, cenario: &Cenario) {
    workbook.editar(aba, |s| {
        s.abrir_transacao();
        for ((r, c), raw) in &cenario.valores { s.set_cell(*r, *c, raw); }
        s.fechar_transacao();
    });
}

/// O "Resumo do cenário": para cada célula de resultado, o valor atual e o
/// valor com cada cenário. Aplica sem histórico e devolve tudo como estava.
/// Resultado: uma linha por célula de resultado, colunas = [atual, cenários…].
pub fn resumo(workbook: &mut Workbook, aba: usize, cenarios: &[Cenario],
    resultados: &[(u32, u32)]) -> Vec<Vec<Value>> {
    let ler = |w: &Workbook| resultados.iter().map(|&(r, c)| w.valor(aba, r, c)).collect::<Vec<_>>();
    let mut colunas = vec![ler(workbook)];
    // Todas as células variáveis de todos os cenários, com o conteúdo de agora.
    let mut originais: Vec<((u32, u32), Option<Snap>)> = Vec::new();
    if let Some(a) = workbook.aba(aba) {
        for cen in cenarios {
            for (p, _) in &cen.valores {
                if !originais.iter().any(|(q, _)| q == p) { originais.push((*p, a.sheet.snapshot(p.0, p.1))); }
            }
        }
    }
    let restaurar = |w: &mut Workbook| w.editar(aba, |s| {
        for ((r, c), snap) in &originais { s.apply(*r, *c, snap.clone(), false); }
    });
    for cen in cenarios {
        restaurar(workbook);
        workbook.editar(aba, |s| {
            for ((r, c), raw) in &cen.valores {
                let estilo = s.snapshot(*r, *c).and_then(|x| x.style);
                s.apply(*r, *c, Some(Snap { raw: raw.clone(), style: estilo }), false);
            }
        });
        colunas.push(ler(workbook));
    }
    restaurar(workbook);
    (0..resultados.len()).map(|i| colunas.iter().map(|col| col[i].clone()).collect()).collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pasta() -> Workbook {
        let mut w = Workbook::new();
        w.escrever_celula(0, 0, 0, "10".into());          // A1 preço
        w.escrever_celula(0, 1, 0, "3".into());           // A2 quantidade
        w.escrever_celula(0, 2, 0, "=A1*A2".into());      // A3 total
        w
    }

    fn cen(nome: &str, a1: &str, a2: &str) -> Cenario {
        Cenario { nome: nome.into(), valores: vec![((0, 0), a1.into()), ((1, 0), a2.into())] }
    }

    #[test]
    fn resumo_calcula_cada_cenario_e_devolve_a_planilha() {
        let mut w = pasta();
        let tabela = resumo(&mut w, 0, &[cen("Alta", "20", "5"), cen("Baixa", "5", "1")], &[(2, 0)]);
        assert_eq!(tabela, vec![vec![Value::Number(30.), Value::Number(100.), Value::Number(5.)]]);
        assert_eq!(w.valor(0, 2, 0), Value::Number(30.));
        assert_eq!(w.valor(0, 0, 0), Value::Number(10.));
    }

    #[test]
    fn mostrar_aplica_com_um_desfazer() {
        let mut w = pasta();
        aplicar(&mut w, 0, &cen("Alta", "20", "5"));
        assert_eq!(w.valor(0, 2, 0), Value::Number(100.));
        assert!(w.aba_mut(0).unwrap().sheet.undo());
        assert_eq!(w.valor(0, 2, 0), Value::Number(30.));
    }
}
