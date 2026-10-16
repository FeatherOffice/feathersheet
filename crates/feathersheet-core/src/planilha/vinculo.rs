//! Referência a outra pasta, como o vínculo externo do Excel (pedido de
//! 24/09/2026): `='[plan_b.plz]Planilha1'!A1` lê a célula A1 da aba
//! Planilha1 do arquivo `plan_b.plz`.
//!
//! O analisador já entende isso: é uma aba entre aspas cujo nome começa com
//! `[arquivo]`. O que mora aqui é o resto:
//!
//! - separar o arquivo da aba ([`separar`]);
//! - guardar as pastas externas, **só com os valores** já calculados de cada
//!   aba ([`retrato_de_valores`]). A fórmula de lá não é recalculada aqui —
//!   e assim uma referência de lá para outra aba nunca é resolvida contra a
//!   pasta errada;
//! - anotar o arquivo que alguém pediu e ainda não está carregado. Quem lê o
//!   disco é a ponte (este crate não lê arquivo — regra 11 do `CLAUDE.md`):
//!   ela pega os pedidos, carrega e devolve com [`Externos::instalar`].
//!
//! O nome do arquivo vai como foi escrito: relativo à pasta do arquivo que o
//! cita, ou caminho absoluto. Quem resolve o caminho é a ponte.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

use crate::celulas::refs::{key_col, key_row, Rect};
use crate::planilha::sheet::Sheet;
use crate::planilha::workbook::Workbook;

/// `[arquivo]Aba` → (`arquivo`, `Aba`). `None` quando o nome não é externo.
pub fn separar(nome: &str) -> Option<(&str, &str)> {
    let resto = nome.strip_prefix('[')?;
    let fim = resto.find(']')?;
    let (arquivo, aba) = (&resto[..fim], &resto[fim + 1..]);
    (!arquivo.is_empty() && !aba.is_empty()).then_some((arquivo, aba))
}

/// A referência escrita na fórmula: sempre entre aspas, por causa dos
/// colchetes (`'[plan_b.plz]Planilha1'!A1:B2`).
pub fn referencia(arquivo: &str, aba: &str, r: &Rect) -> String {
    let nome = format!("[{arquivo}]{aba}").replace('\'', "''");
    let a1 = crate::celulas::refs::cell_name(r.r0, r.c0);
    if r.r0 == r.r1 && r.c0 == r.c1 {
        format!("'{nome}'!{a1}")
    } else {
        format!("'{nome}'!{a1}:{}", crate::celulas::refs::cell_name(r.r1, r.c1))
    }
}

/// As abas de uma pasta só com os valores (cada fórmula vira o resultado
/// dela, calculado na pasta de origem).
pub fn retrato_de_valores(pasta: &Workbook) -> Vec<Sheet> {
    (0..pasta.len())
        .filter_map(|i| {
            let viva = pasta.aba(i)?;
            let mut copia = Sheet::new(viva.sheet.name.clone());
            let chaves = viva.sheet.chaves_no_retangulo(&Rect::new(0, 0, u32::MAX - 1, u32::MAX - 1));
            for k in chaves {
                let (r, c) = (key_row(k), key_col(k));
                copia.set_cell_valor(r, c, pasta.valor(i, r, c));
            }
            copia.rastro.borrow_mut().desconhecer();
            Some(copia)
        })
        .collect()
}

/// Se duas listas de abas têm os mesmos nomes e os mesmos valores.
fn mesmas_abas(a: &[Sheet], b: &[Sheet]) -> bool {
    let tudo = Rect::new(0, 0, u32::MAX - 1, u32::MAX - 1);
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            if x.name != y.name {
                return false;
            }
            let (kx, ky) = (x.chaves_no_retangulo(&tudo), y.chaves_no_retangulo(&tudo));
            kx == ky && kx.iter().all(|&k| x.get_value(key_row(k), key_col(k)) == y.get_value(key_row(k), key_col(k)))
        })
}

/// As pastas externas de uma pasta, pelo nome do arquivo em minúsculas.
#[derive(Default)]
pub struct Externos {
    /// Pela chave em minúsculas: o nome como foi escrito (o disco do Linux
    /// diferencia maiúscula) e as abas.
    livros: HashMap<String, (String, Vec<Sheet>)>,
    /// Arquivos citados que ainda não foram carregados. `RefCell` porque é
    /// no meio do cálculo (com `&self`) que se descobre a falta.
    pedidos: RefCell<BTreeSet<String>>,
}

impl Externos {
    /// A aba externa com esse nome (`[arquivo]Aba`), se o arquivo já está
    /// carregado; senão anota o pedido. `None` também para nome que não é
    /// externo.
    pub fn aba(&self, nome: &str) -> Option<&Sheet> {
        let (arquivo, aba) = separar(nome)?;
        let chave = arquivo.to_lowercase();
        match self.livros.get(&chave) {
            Some((_, abas)) => {
                let aba = aba.to_lowercase();
                abas.iter().find(|s| s.name.to_lowercase() == aba)
            }
            None => {
                self.pedidos.borrow_mut().insert(arquivo.to_string());
                None
            }
        }
    }

    /// Os arquivos que o cálculo pediu desde a última pergunta.
    pub fn tirar_pedidos(&self) -> Vec<String> {
        std::mem::take(&mut *self.pedidos.borrow_mut()).into_iter().collect()
    }

    /// Os arquivos já carregados (como foram escritos na chave), para a ponte
    /// conferir se mudaram no disco.
    pub fn carregados(&self) -> Vec<String> {
        self.livros.values().map(|(nome, _)| nome.clone()).collect()
    }

    /// Troca valores avulsos de uma aba já carregada — o que a outra janela
    /// manda na hora da edição, sem esperar o arquivo ir ao disco. Responde
    /// se algum valor MUDOU: igual não conta, senão duas pastas que se citam
    /// ficariam avisando uma à outra para sempre. `false` também quando o
    /// arquivo ou a aba não estão carregados.
    pub fn atualizar_celulas(&mut self, arquivo: &str, aba: &str, celulas: &[(u32, u32, crate::celulas::values::Value)]) -> bool {
        let Some((_, abas)) = self.livros.get_mut(&arquivo.to_lowercase()) else { return false };
        let aba = aba.to_lowercase();
        let Some(s) = abas.iter_mut().find(|s| s.name.to_lowercase() == aba) else { return false };
        let mut mudou = false;
        for (r, c, v) in celulas {
            if s.get_value(*r, *c) != *v {
                s.set_cell_valor(*r, *c, v.clone());
                mudou = true;
            }
        }
        mudou
    }

    /// Põe (ou troca) a pasta de um arquivo. Responde se mudou alguma coisa
    /// (pelo mesmo motivo do [`Self::atualizar_celulas`]).
    pub fn instalar(&mut self, arquivo: &str, abas: Vec<Sheet>) -> bool {
        let chave = arquivo.to_lowercase();
        let igual = self.livros.get(&chave).is_some_and(|(_, velhas)| mesmas_abas(velhas, &abas));
        self.livros.insert(chave, (arquivo.to_string(), abas));
        !igual
    }

    pub fn esquecer_tudo(&mut self) {
        self.livros.clear();
        self.pedidos.borrow_mut().clear();
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::celulas::values::Value;

    #[test]
    fn separa_arquivo_e_aba() {
        assert_eq!(separar("[plan_b.plz]Planilha1"), Some(("plan_b.plz", "Planilha1")));
        assert_eq!(separar("[/home/x/plan b.plz]Meu Plano"), Some(("/home/x/plan b.plz", "Meu Plano")));
        assert_eq!(separar("Planilha1"), None);
        assert_eq!(separar("[]Aba"), None);
        assert_eq!(separar("[x.plz]"), None);
    }

    #[test]
    fn escreve_a_referencia_entre_aspas() {
        assert_eq!(referencia("plan_b.plz", "Planilha1", &Rect::new(0, 0, 0, 0)), "'[plan_b.plz]Planilha1'!A1");
        assert_eq!(referencia("d'água.plz", "P", &Rect::new(1, 1, 3, 2)), "'[d''água.plz]P'!B2:C4");
    }

    /// A outra janela manda só as células que mudaram: a A recalcula sem
    /// recarregar a pasta inteira.
    #[test]
    fn valores_avulsos_da_outra_janela() {
        let mut b = Workbook::new();
        b.escrever_celula(0, 0, 0, "10".into());
        let mut a = Workbook::new();
        a.escrever_celula(0, 0, 0, "='[plan_b.plz]Planilha1'!A1+1".into());
        let _ = a.valor(0, 0, 0);
        a.instalar_externo("plan_b.plz", retrato_de_valores(&b));
        assert_eq!(a.valor(0, 0, 0), Value::Number(11.0));
        assert!(a.atualizar_externo("plan_b.plz", "planilha1", &[(0, 0, Value::Number(99.0))]));
        assert_eq!(a.valor(0, 0, 0), Value::Number(100.0));
        // O mesmo valor de novo não é mudança (duas pastas que se citam
        // não podem ficar avisando uma à outra para sempre).
        let versao = a.versao(0);
        assert!(!a.atualizar_externo("plan_b.plz", "planilha1", &[(0, 0, Value::Number(99.0))]));
        assert_eq!(a.versao(0), versao);
        assert!(!a.instalar_externo("plan_b.plz", retrato_de_valores(&{
            let mut b2 = Workbook::new();
            b2.escrever_celula(0, 0, 0, "99".into());
            b2
        })), "a mesma pasta de novo não muda nada");
        assert!(!a.atualizar_externo("outro.plz", "Planilha1", &[]), "arquivo não carregado");
    }

    /// De ponta a ponta: a pasta A lê a B, que chega depois (como a ponte faz).
    #[test]
    fn a_formula_le_a_outra_pasta_quando_ela_chega() {
        let mut b = Workbook::new();
        b.escrever_celula(0, 0, 0, "10".into());
        b.escrever_celula(0, 1, 0, "=A1*3".into());
        let mut a = Workbook::new();
        a.escrever_celula(0, 0, 0, "='[plan_b.plz]Planilha1'!A2+1".into());
        a.escrever_celula(0, 1, 0, "=SOMA('[plan_b.plz]Planilha1'!A1:A2)".into());
        // Antes de carregar: #REF!, e o pedido fica anotado.
        assert!(matches!(a.valor(0, 0, 0), Value::Error(_)));
        assert_eq!(a.externos_pedidos(), vec!["plan_b.plz".to_string()]);
        a.instalar_externo("plan_b.plz", retrato_de_valores(&b));
        assert_eq!(a.valor(0, 0, 0), Value::Number(31.0));
        assert_eq!(a.valor(0, 1, 0), Value::Number(40.0));
        // A B mudou no disco: a nova versão chega e a A recalcula.
        b.escrever_celula(0, 0, 0, "20".into());
        a.instalar_externo("plan_b.plz", retrato_de_valores(&b));
        assert_eq!(a.valor(0, 0, 0), Value::Number(61.0));
        assert_eq!(a.valor(0, 1, 0), Value::Number(80.0));
    }
}
