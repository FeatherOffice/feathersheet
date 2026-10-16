//! Faixa lida de uma vez, para as funções de critério (`SOMASES`,
//! `CONT.SES`, `MÉDIASES`…) não perguntarem célula por célula.
//!
//! Antes, cada `SOMASES` de 1 milhão de linhas fazia 3 milhões de perguntas
//! `get_value` — cada uma resolvendo o nome da aba, procurando em mapas e
//! clonando o valor —, e a aba Resumo, com ~150 delas, levava 84 s (release)
//! travando a janela. Agora cada coluna é lida uma vez e fica guardada até a
//! aba mudar ([`Faixas::geracao`]).

use std::collections::HashMap;
use std::sync::Arc;

use crate::celulas::values::Value;

/// Abaixo disto ler célula por célula já é barato. Pequeno de propósito:
/// a lista de 200 produtos de um `PROCX` é lida por um milhão de linhas.
pub(crate) const MINIMO: u64 = 128;
/// Uma faixa maior que isto não é guardada (memória).
pub(crate) const MAXIMO: u64 = 4_000_000;
/// Soma das células guardadas numa aba; passou, esvazia tudo.
pub(crate) const TOTAL: usize = 12_000_000;

/// Os valores de um retângulo, linha a linha. Fora do retângulo guardado
/// (além da última linha/coluna usada da aba) a célula é vazia.
#[derive(Debug)]
pub struct FaixaLida {
    r0: u32,
    c0: u32,
    linhas: u32,
    colunas: u32,
    valores: Vec<Value>,
}

static VAZIO: Value = Value::Empty;

impl FaixaLida {
    pub(crate) fn nova(r0: u32, c0: u32, linhas: u32, colunas: u32, valores: Vec<Value>) -> Self {
        debug_assert_eq!(valores.len() as u64, linhas as u64 * colunas as u64);
        FaixaLida { r0, c0, linhas, colunas, valores }
    }

    /// Cada valor não vazio, com a linha e a coluna dele na aba.
    pub fn para_cada(&self, visit: &mut dyn FnMut(&Value, u32, u32)) {
        if self.colunas == 0 {
            return;
        }
        for (i, v) in self.valores.iter().enumerate() {
            if !matches!(v, Value::Empty) {
                let (r, c) = (i / self.colunas as usize, i % self.colunas as usize);
                visit(v, self.r0 + r as u32, self.c0 + c as u32);
            }
        }
    }

    /// O valor da célula (`row`, `col`) da aba — não da faixa.
    pub fn valor(&self, row: u32, col: u32) -> &Value {
        let (Some(r), Some(c)) = (row.checked_sub(self.r0), col.checked_sub(self.c0)) else {
            return &VAZIO;
        };
        if r >= self.linhas || c >= self.colunas {
            return &VAZIO;
        }
        &self.valores[r as usize * self.colunas as usize + c as usize]
    }
}

/// As faixas guardadas de uma aba.
#[derive(Default)]
pub(crate) struct Faixas {
    /// A geração da aba quando estas faixas foram lidas.
    pub geracao: u64,
    pub celulas: usize,
    pub mapa: HashMap<(u32, u32, u32, u32), Arc<FaixaLida>>,
}
