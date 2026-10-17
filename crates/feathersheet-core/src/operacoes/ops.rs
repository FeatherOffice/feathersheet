//! Operações na planilha em Rust: localizar e substituir, estatísticas,
//! região atual, remover duplicados…
//!
//! Tradução do `ops.ts`. Quem precisa do valor calculado de uma célula recebe
//! um leitor (`valor`), e não lê o `Sheet` direto: uma fórmula que cita outra
//! aba só calcula certo com a pasta em volta (`Workbook::valor`), que é o que
//! a ponte passa. Os atalhos sem `_com` leem o `Sheet` sozinho (testes).

mod autosoma;
mod busca;
mod cabecalho;
mod comum;
mod dividir_texto;
mod estatistica;
mod faixas_ocultas;
mod limpeza;
mod ordenacao;
pub(crate) mod regiao;
mod repetidas;

pub use autosoma::*;
pub use busca::*;
pub use cabecalho::*;
pub use dividir_texto::*;
pub use estatistica::*;
pub use faixas_ocultas::*;
pub use limpeza::*;
pub use ordenacao::*;
pub use regiao::*;
pub use repetidas::*;
#[allow(unused_imports)]
use comum::*;

// Os módulos de teste filhos enxergam estes nomes por `use super::*`.
#[cfg(test)]
#[allow(unused_imports)]
use {
    crate::celulas::estilo_parcial::{nome_do_formato, Campo, EstiloParcial},
    crate::celulas::input::{date_to_serial, parse_number_str},
    crate::celulas::refs::{cell_name, Rect, MAX_ROWS},
    crate::celulas::style::NumberFormat as FormatoDeEstilo,
    crate::celulas::values::{to_text, Value},
    crate::planilha::data_table::Cru,
    crate::planilha::sheet::Sheet,
    crate::planilha::textos_crus::TextosCrus,
    std::collections::{HashMap, HashSet},
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod testes_duplicados;

#[cfg(test)]
mod testes_rodape;

#[cfg(test)]
mod testes_regiao;

#[cfg(test)]
mod testes_linhas;

#[cfg(test)]
mod testes_busca_rapida;

#[cfg(test)]
mod testes_ordenar_permutando;
#[cfg(test)]
mod testes_cabecalho;

#[cfg(test)]
mod testes_divisao_lote;
