//! Agregado por critério: uma passada pelas colunas serve a todas as
//! `SOMASES`/`CONT.SES`/`MÉDIASES` que leem as mesmas faixas.
//!
//! A aba Resumo da Planilha Pesada tem ~150 `SOMASES(Dados!I; Dados!C; $A2;
//! Dados!J; 1)`: só muda o critério (`$A2`). Cada uma varria 1 milhão de
//! linhas — ~25 ms, 4,4 s a aba inteira, mesmo com a faixa já lida de uma
//! vez ([`crate::planilha::faixa_lida`]). Aqui a varredura acontece **uma
//! vez por conjunto de faixas**: cada linha é posta no grupo da tupla dos
//! seus valores de critério, e o grupo acumula soma, contagem, máximo e
//! mínimo da faixa somada. Depois cada fórmula é uma consulta ao mapa.
//!
//! Só critério de **igualdade simples** usa o agregado (número diferente de
//! zero, ou texto sem operador e sem curinga) — é o que
//! [`crate::funcoes::fn_helpers::make_predicate`] trata como "igualdade", com
//! a mesma regra de comparação (texto sem ligar para maiúscula, número por
//! valor). `">10"`, `"a*"`, vazio, zero e lógico continuam no caminho célula
//! por célula, que já estava certo.
//!
//! Frente de velocidade V2 do plano (`docs/planejamento/PLANO-MEMORIA-400MB.md`).

use std::collections::HashMap;
use std::sync::Arc;

use crate::celulas::values::Value;
use crate::planilha::data_table::Cru;

/// Abaixo disto a varredura direta já é barata; não vale montar o mapa.
pub(crate) const MINIMO: u64 = 2_048;
/// Mais grupos que isto é uma coluna de valores quase únicos: o mapa
/// ficaria do tamanho da coluna sem servir a ninguém.
pub(crate) const MAX_GRUPOS: usize = 262_144;

/// O valor de um critério (ou de uma célula) na forma em que se compara.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ChaveCriterio {
    /// Bits do `f64`, com `-0` normalizado para `0`.
    Numero(u64),
    /// Texto em minúsculas (a igualdade de critério ignora maiúsculas).
    /// `Arc` para a chave de uma célula custar um incremento, não uma cópia.
    Texto(Arc<str>),
}

fn bits(n: f64) -> u64 {
    if n == 0.0 { 0.0f64.to_bits() } else { n.to_bits() }
}

fn minusculas(s: &str) -> Arc<str> {
    if s.is_ascii() {
        Arc::from(s.to_ascii_lowercase())
    } else {
        Arc::from(s.chars().flat_map(char::to_lowercase).collect::<String>())
    }
}

/// Lê a chave de cada célula de critério. Quem monta o agregado implementa
/// sobre o armazenamento da aba (`Sheet`), lendo o texto pelo id do pool
/// quando dá — a mesma string de 1 milhão de linhas passa para minúscula uma
/// vez, não um milhão.
pub(crate) struct LeitorChaves<'a> {
    /// Chave já montada por id de texto no pool da aba.
    por_id: HashMap<i32, ChaveCriterio>,
    pub texto_do_pool: &'a dyn Fn(i32) -> &'a str,
}

impl<'a> LeitorChaves<'a> {
    pub(crate) fn novo(texto_do_pool: &'a dyn Fn(i32) -> &'a str) -> Self {
        LeitorChaves { por_id: HashMap::new(), texto_do_pool }
    }

    /// A chave de um valor cru (número ou id de texto no pool).
    /// `literal`: o cru veio da coluna de dados, onde `VERDADEIRO`/`FALSO`
    /// no pool são a célula lógica (não casa com critério indexável); o
    /// resultado de fórmula guarda lógico à parte, então ali é texto mesmo.
    pub(crate) fn do_cru(&mut self, cru: Cru, literal: bool) -> Result<Option<ChaveCriterio>, ()> {
        match cru {
            Cru::Numero(n) if n.is_nan() => Err(()),
            Cru::Numero(n) => Ok(Some(ChaveCriterio::Numero(bits(n)))),
            Cru::Texto(id) => {
                if let Some(c) = self.por_id.get(&id) {
                    return Ok(Some(c.clone()));
                }
                let texto = (self.texto_do_pool)(id);
                if texto.is_empty() || (literal && (texto == "VERDADEIRO" || texto == "FALSO")) {
                    return Ok(None);
                }
                let chave = ChaveCriterio::Texto(minusculas(texto));
                self.por_id.insert(id, chave.clone());
                Ok(Some(chave))
            }
        }
    }
}

impl ChaveCriterio {
    /// A chave de um critério, quando ele é indexável: número finito e
    /// diferente de zero (zero também casa com célula vazia), ou texto não
    /// vazio sem operador (`>`, `<`, `=`) nem curinga (`*`, `?`).
    pub fn do_criterio(v: &Value) -> Option<Self> {
        match v {
            Value::Number(n) if n.is_finite() && *n != 0.0 => Some(ChaveCriterio::Numero(bits(*n))),
            Value::Text(t) => {
                let aparado = t.trim();
                if aparado.is_empty() || t.contains('*') || t.contains('?')
                    || aparado.starts_with(['<', '>', '=']) {
                    return None;
                }
                Some(ChaveCriterio::Texto(minusculas(t)))
            }
            _ => None,
        }
    }

    /// A chave de um valor procurado por igualdade exata (`PROCX` modo 0):
    /// como [`Self::do_criterio`], mas sem a regra de operador e curinga —
    /// ali o texto é literal. Zero e vazio ficam de fora (casam entre si).
    pub fn exata(v: &Value) -> Option<Self> {
        match v {
            Value::Number(n) if n.is_finite() && *n != 0.0 => Some(ChaveCriterio::Numero(bits(*n))),
            Value::Text(t) if !t.is_empty() => Some(ChaveCriterio::Texto(minusculas(t))),
            _ => None,
        }
    }

    /// A chave de uma célula de critério. `Ok(None)`: a célula não casa com
    /// critério indexável nenhum (vazia, lógico, erro). `Err(())`: um `NaN`,
    /// que casa com qualquer número — o agregado não serve para esta faixa.
    pub(crate) fn da_celula(v: &Value) -> Result<Option<Self>, ()> {
        match v {
            Value::Number(n) if n.is_nan() => Err(()),
            Value::Number(n) => Ok(Some(ChaveCriterio::Numero(bits(*n)))),
            Value::Text(t) if !t.is_empty() => Ok(Some(ChaveCriterio::Texto(minusculas(t)))),
            _ => Ok(None),
        }
    }
}

/// O que se acumulou das linhas de um grupo.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Grupo {
    /// Linhas cujos critérios casaram (o `CONT.SES`).
    pub linhas: u32,
    /// Dessas, quantas têm número na faixa agregada (o `MÉDIASES` divide por isto).
    pub numeros: u32,
    pub soma: f64,
    pub maximo: f64,
    pub minimo: f64,
}

impl Grupo {
    fn acrescentar(&mut self, numero: Option<f64>) {
        self.linhas += 1;
        if let Some(n) = numero {
            if self.numeros == 0 {
                self.maximo = n;
                self.minimo = n;
            } else {
                self.maximo = self.maximo.max(n);
                self.minimo = self.minimo.min(n);
            }
            self.numeros += 1;
            self.soma += n;
        }
    }
}

/// Os grupos de um conjunto de faixas de critério (e de uma faixa agregada,
/// quando há).
#[derive(Debug, Default)]
pub struct Agregado {
    grupos: HashMap<Vec<ChaveCriterio>, Grupo>,
}

impl Agregado {
    /// O grupo da tupla de critérios; vazio (zeros) quando nenhuma linha casou.
    pub fn grupo(&self, chaves: &[ChaveCriterio]) -> Grupo {
        self.grupos.get(chaves).copied().unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.grupos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.grupos.is_empty()
    }

    /// Monta o agregado lendo cada posição `(linha, coluna)` de
    /// `0..linhas × 0..colunas`: `chave(i, r_off, c_off)` dá a chave da
    /// faixa de critério `i` (`Err`: um `NaN`, desiste) e `numero(r_off,
    /// c_off)` o número da faixa agregada, se for número. `None` quando um
    /// `NaN` aparece ou há grupos demais.
    pub(crate) fn montar(
        n_criterios: usize,
        tem_soma: bool,
        linhas: u32,
        colunas: u32,
        mut chave: impl FnMut(usize, u32, u32) -> Result<Option<ChaveCriterio>, ()>,
        mut numero: impl FnMut(u32, u32) -> Option<f64>,
    ) -> Option<Self> {
        let mut grupos: HashMap<Vec<ChaveCriterio>, Grupo> = HashMap::new();
        let mut chaves: Vec<ChaveCriterio> = Vec::with_capacity(n_criterios);
        for r in 0..linhas {
            'posicao: for c in 0..colunas {
                chaves.clear();
                for i in 0..n_criterios {
                    match chave(i, r, c) {
                        Ok(Some(chave)) => chaves.push(chave),
                        Ok(None) => continue 'posicao,
                        Err(()) => return None,
                    }
                }
                let valor = if tem_soma { numero(r, c) } else { None };
                if let Some(grupo) = grupos.get_mut(chaves.as_slice()) {
                    grupo.acrescentar(valor);
                } else {
                    if grupos.len() >= MAX_GRUPOS {
                        return None;
                    }
                    let mut grupo = Grupo::default();
                    grupo.acrescentar(valor);
                    grupos.insert(chaves.clone(), grupo);
                }
            }
        }
        Some(Agregado { grupos })
    }
}

/// Um retângulo sem o nome da aba, como chave.
pub(crate) type Area = (u32, u32, u32, u32);

/// Os agregados guardados de uma aba, válidos enquanto ela não muda.
#[derive(Default)]
pub(crate) struct Agregados {
    /// A geração da aba quando foram montados (ver `Sheet::geracao`).
    pub geracao: u64,
    /// `(faixa agregada, faixas de critério) → agregado`.
    pub mapa: HashMap<(Option<Area>, Vec<Area>), Arc<Agregado>>,
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn criterio_indexavel_e_o_de_igualdade_simples() {
        assert_eq!(ChaveCriterio::do_criterio(&Value::Number(2.0)), Some(ChaveCriterio::Numero(2.0f64.to_bits())));
        assert_eq!(ChaveCriterio::do_criterio(&Value::Number(0.0)), None);
        assert_eq!(ChaveCriterio::do_criterio(&Value::Number(f64::NAN)), None);
        assert_eq!(ChaveCriterio::do_criterio(&Value::Text("Sul".into())), Some(ChaveCriterio::Texto(Arc::from("sul"))));
        assert_eq!(ChaveCriterio::do_criterio(&Value::Text("ÁGUA".into())), Some(ChaveCriterio::Texto(Arc::from("água"))));
        assert_eq!(ChaveCriterio::do_criterio(&Value::Text(">10".into())), None);
        assert_eq!(ChaveCriterio::do_criterio(&Value::Text("a*".into())), None);
        assert_eq!(ChaveCriterio::do_criterio(&Value::Text("".into())), None);
        assert_eq!(ChaveCriterio::do_criterio(&Value::Bool(true)), None);
        assert_eq!(ChaveCriterio::do_criterio(&Value::Empty), None);
    }

    #[test]
    fn celula_casa_com_o_criterio_pela_mesma_regra() {
        assert_eq!(ChaveCriterio::da_celula(&Value::Number(-0.0)), Ok(Some(ChaveCriterio::Numero(0))));
        assert_eq!(ChaveCriterio::da_celula(&Value::Text("SUL".into())), Ok(Some(ChaveCriterio::Texto(Arc::from("sul")))));
        assert_eq!(ChaveCriterio::da_celula(&Value::Empty), Ok(None));
        assert_eq!(ChaveCriterio::da_celula(&Value::Bool(true)), Ok(None));
        assert_eq!(ChaveCriterio::da_celula(&Value::Number(f64::NAN)), Err(()));
    }

    #[test]
    fn agrupa_e_acumula() {
        // Critério 0: região; critério 1: ano; soma: valor.
        let regiao = ["Sul", "Norte", "sul", "Sul", "Norte", ""];
        let ano = [1.0, 1.0, 1.0, 2.0, 1.0, 1.0];
        let valor = [Value::Number(10.0), Value::Number(5.0), Value::Number(7.0), Value::Number(100.0), Value::Text("x".into()), Value::Number(1.0)];
        let ag = Agregado::montar(2, true, 6, 1,
            |i, r, _| ChaveCriterio::da_celula(&match i {
                0 => Value::Text(regiao[r as usize].into()),
                _ => Value::Number(ano[r as usize]),
            }),
            |r, _| match valor[r as usize] { Value::Number(n) => Some(n), _ => None },
        ).expect("monta");
        let sul1 = ag.grupo(&[ChaveCriterio::Texto(Arc::from("sul")), ChaveCriterio::Numero(1.0f64.to_bits())]);
        assert_eq!(sul1, Grupo { linhas: 2, numeros: 2, soma: 17.0, maximo: 10.0, minimo: 7.0 });
        let norte1 = ag.grupo(&[ChaveCriterio::Texto(Arc::from("norte")), ChaveCriterio::Numero(1.0f64.to_bits())]);
        assert_eq!(norte1, Grupo { linhas: 2, numeros: 1, soma: 5.0, maximo: 5.0, minimo: 5.0 });
        let nada = ag.grupo(&[ChaveCriterio::Texto(Arc::from("leste")), ChaveCriterio::Numero(1.0f64.to_bits())]);
        assert_eq!(nada, Grupo::default());
        // A linha com região vazia não entra em grupo nenhum.
        assert_eq!(ag.len(), 3);
    }

    #[test]
    fn nan_desiste_do_agregado() {
        let ag = Agregado::montar(1, false, 3, 1,
            |_, r, _| ChaveCriterio::da_celula(&if r == 1 { Value::Number(f64::NAN) } else { Value::Number(1.0) }),
            |_, _| None);
        assert!(ag.is_none());
    }

    #[test]
    fn leitor_de_chaves_reaproveita_por_id_e_respeita_logico() {
        let textos = ["Sul", "VERDADEIRO", ""];
        let texto = |id: i32| textos[id as usize];
        let mut leitor = LeitorChaves::novo(&texto);
        assert_eq!(leitor.do_cru(Cru::Texto(0), true), Ok(Some(ChaveCriterio::Texto(Arc::from("sul")))));
        assert_eq!(leitor.do_cru(Cru::Texto(0), true), Ok(Some(ChaveCriterio::Texto(Arc::from("sul")))));
        assert_eq!(leitor.do_cru(Cru::Texto(1), true), Ok(None));
        assert_eq!(leitor.do_cru(Cru::Texto(1), false), Ok(Some(ChaveCriterio::Texto(Arc::from("verdadeiro")))));
        assert_eq!(leitor.do_cru(Cru::Texto(2), false), Ok(None));
        assert_eq!(leitor.do_cru(Cru::Numero(-0.0), true), Ok(Some(ChaveCriterio::Numero(0))));
        assert_eq!(leitor.do_cru(Cru::Numero(f64::NAN), true), Err(()));
    }
}
