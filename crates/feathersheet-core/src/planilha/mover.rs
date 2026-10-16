//! Mover um bloco de células arrastando pela borda, como no Excel.
//!
//! Funcionalidade nova, só do Rust (não existe no TypeScript). O que o
//! Excel faz, e o que aqui é feito igual:
//!
//! - o conteúdo e o estilo direto vão para o destino, e a origem fica vazia;
//! - **toda fórmula da pasta** que apontava para uma célula movida passa a
//!   apontar para onde ela foi — inclusive as fórmulas movidas, quando leem
//!   uma vizinha que foi junto;
//! - a fórmula movida que lê fora do bloco continua lendo a mesma célula
//!   (não é o colar, que desloca a referência relativa);
//! - a referência a uma célula do destino que foi sobrescrita vira `#REF!`.
//!
//! Intervalo (`A1:A10`) é tratado inteiro, e é por isso que este módulo não
//! usa o [`map_references`] direto, que vê as duas pontas separadas: o
//! intervalo anda só quando está todo dentro do bloco movido, e vira `#REF!`
//! só quando está todo dentro do que foi sobrescrito. Mover só `A1` não
//! transforma `A1:A10` em `C1:A10`. (O Excel estica o intervalo quando se
//! move uma ponta dele; aqui ele fica como estava — mais previsível.)
//!
//! Ficam de fora, por ora: mesclas, tabelas, gráficos, regras de formatação
//! condicional e validação que apontem para o bloco não acompanham.

use crate::formula::formula_text::{format_ref, map_references, ParsedRef};
use crate::celulas::refs::Rect;

/// Um bloco `origem` (já normalizado) da aba `aba` indo `dr` linhas e `dc`
/// colunas adiante.
#[derive(Debug, Clone)]
pub struct Movimento {
    pub aba: String,
    pub origem: Rect,
    pub dr: i32,
    pub dc: i32,
}

/// Marca que substitui cada referência no primeiro passo da reescrita.
const MARCA: char = '\u{2}';

impl Movimento {
    /// O retângulo de destino, ou `None` quando sairia da planilha.
    pub fn destino(&self) -> Option<Rect> {
        let mover = |p: u32, d: i32| u32::try_from(p as i64 + d as i64).ok();
        Some(Rect::new(
            mover(self.origem.r0, self.dr)?,
            mover(self.origem.c0, self.dc)?,
            mover(self.origem.r1, self.dr)?,
            mover(self.origem.c1, self.dc)?,
        ))
    }

    fn na_aba(&self, nome: &str) -> bool {
        nome.eq_ignore_ascii_case(&self.aba)
    }

    /// O texto novo de uma fórmula que mora na aba `propria`. Devolve o
    /// mesmo texto quando nenhuma referência muda — o texto não é
    /// renormalizado à toa.
    pub fn reescrever(&self, raw: &str, propria: &str) -> String {
        if !raw.starts_with('=') || raw.contains(MARCA) {
            return raw.to_string();
        }
        let Some(destino) = self.destino() else {
            return raw.to_string();
        };

        // 1) cada referência vira uma marca numerada, para enxergar quais
        //    formam um intervalo (`marca:marca`).
        let mut refs: Vec<ParsedRef> = Vec::new();
        let marcado = map_references(raw, |r| {
            refs.push(r);
            format!("{MARCA}{}{MARCA}", refs.len() - 1)
        });
        if refs.is_empty() {
            return raw.to_string();
        }

        // 2) os pedaços: texto entre as marcas e o número de cada marca.
        let mut pedacos: Vec<(String, Option<usize>)> = Vec::new();
        let mut resto = marcado.as_str();
        while let Some(i) = resto.find(MARCA) {
            let fim = resto[i + 1..].find(MARCA).expect("toda marca fecha") + i + 1;
            pedacos.push((resto[..i].to_string(), None));
            pedacos.push((String::new(), Some(resto[i + 1..fim].parse().expect("número da marca"))));
            resto = &resto[fim + 1..];
        }
        pedacos.push((resto.to_string(), None));

        // 3) o que cada referência vira. Um intervalo é uma marca, um `:`
        //    colado e outra marca.
        let mut novo: Vec<Option<String>> = vec![None; refs.len()];
        // A segunda ponta de um intervalo que virou `#REF!`: some junto
        // com o `:`, e `A1:B2` vira um `#REF!` só.
        let mut pular: Vec<bool> = vec![false; refs.len()];
        let dentro = |r: &Rect, row: i32, col: i32| {
            row >= r.r0 as i32 && row <= r.r1 as i32 && col >= r.c0 as i32 && col <= r.c1 as i32
        };
        let mut i = 0;
        while i < pedacos.len() {
            let Some(a) = pedacos[i].1 else {
                i += 1;
                continue;
            };
            let intervalo = pedacos.get(i + 1).is_some_and(|p| p.0 == ":" && p.1.is_none())
                && pedacos.get(i + 2).and_then(|p| p.1).is_some();
            let aba = refs[a].sheet.as_deref().unwrap_or(propria);
            if !self.na_aba(aba) {
                i += if intervalo { 3 } else { 1 };
                continue;
            }
            if intervalo {
                let b = pedacos[i + 2].1.expect("conferido acima");
                let (ra, rb) = (&refs[a], &refs[b]);
                let (r0, r1) = (ra.row.min(rb.row), ra.row.max(rb.row));
                let (c0, c1) = (ra.col.min(rb.col), ra.col.max(rb.col));
                if dentro(&self.origem, r0, c0) && dentro(&self.origem, r1, c1) {
                    novo[a] = Some(self.deslocada(ra));
                    novo[b] = Some(self.deslocada(rb));
                } else if dentro(&destino, r0, c0) && dentro(&destino, r1, c1) && !self.toca_origem(r0, c0, r1, c1) {
                    novo[a] = Some("#REF!".into());
                    pular[b] = true;
                }
                i += 3;
            } else {
                let r = &refs[a];
                if dentro(&self.origem, r.row, r.col) {
                    novo[a] = Some(self.deslocada(r));
                } else if dentro(&destino, r.row, r.col) {
                    novo[a] = Some("#REF!".into());
                }
                i += 1;
            }
        }
        if novo.iter().all(Option::is_none) {
            return raw.to_string();
        }

        // 4) remonta.
        let mut out = String::with_capacity(raw.len() + 8);
        for (n, (texto, marca)) in pedacos.iter().enumerate() {
            match marca {
                Some(m) if pular[*m] => {}
                Some(m) => match &novo[*m] {
                    Some(t) => out.push_str(t),
                    None => out.push_str(&format_ref(&refs[*m])),
                },
                None => {
                    let antes_de_pulada = pedacos.get(n + 1).and_then(|p| p.1).is_some_and(|m| pular[m]);
                    if !(antes_de_pulada && texto == ":") {
                        out.push_str(texto);
                    }
                }
            }
        }
        out
    }

    fn deslocada(&self, r: &ParsedRef) -> String {
        format_ref(&ParsedRef { row: r.row + self.dr, col: r.col + self.dc, ..r.clone() })
    }

    fn toca_origem(&self, r0: i32, c0: i32, r1: i32, c1: i32) -> bool {
        let o = &self.origem;
        r0 <= o.r1 as i32 && r1 >= o.r0 as i32 && c0 <= o.c1 as i32 && c1 >= o.c0 as i32
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// `A1:B2` (linhas 0–1, colunas 0–1) da `Plan1` indo para `D5`.
    fn mov() -> Movimento {
        Movimento { aba: "Plan1".into(), origem: Rect::new(0, 0, 1, 1), dr: 4, dc: 3 }
    }

    #[test]
    fn referencia_ao_bloco_acompanha() {
        let m = mov();
        assert_eq!(m.reescrever("=A1+B2*2", "Plan1"), "=D5+E6*2");
        assert_eq!(m.reescrever("=$A$1+B$2", "Plan1"), "=$D$5+E$6");
        assert_eq!(m.reescrever("=SOMA(A1:B2)", "Plan1"), "=SOMA(D5:E6)");
    }

    #[test]
    fn referencia_de_fora_fica() {
        let m = mov();
        assert_eq!(m.reescrever("=C1+A3", "Plan1"), "=C1+A3");
        // Texto entre aspas não é referência.
        assert_eq!(m.reescrever("=\"A1\"&C1", "Plan1"), "=\"A1\"&C1");
    }

    #[test]
    fn intervalo_parcial_nao_se_desmancha() {
        let m = mov();
        assert_eq!(m.reescrever("=SOMA(A1:A10)", "Plan1"), "=SOMA(A1:A10)");
        assert_eq!(m.reescrever("=SOMA(A1:A2)+A1", "Plan1"), "=SOMA(D5:D6)+D5");
    }

    #[test]
    fn so_a_aba_do_movimento() {
        let m = mov();
        assert_eq!(m.reescrever("=A1", "Outra"), "=A1");
        assert_eq!(m.reescrever("=Plan1!A1+A1", "Outra"), "=Plan1!D5+A1");
        assert_eq!(m.reescrever("=plan1!A1:B2", "Outra"), "=plan1!D5:E6");
        assert_eq!(m.reescrever("='Meu Plano'!A1", "Plan1"), "='Meu Plano'!A1");
    }

    #[test]
    fn destino_sobrescrito_vira_ref() {
        let m = mov();
        assert_eq!(m.reescrever("=E6+1", "Plan1"), "=#REF!+1");
        assert_eq!(m.reescrever("=SOMA(D5:E6)", "Plan1"), "=SOMA(#REF!)");
        // Intervalo que só encosta no destino fica.
        assert_eq!(m.reescrever("=SOMA(D5:D20)", "Plan1"), "=SOMA(D5:D20)");
    }

    #[test]
    fn bloco_sobreposto_ao_destino() {
        // A1:A3 descendo uma linha: A2 e A3 são origem e destino ao mesmo
        // tempo — quem lê A2 segue a célula (vira A3), não vira #REF!.
        let m = Movimento { aba: "P".into(), origem: Rect::new(0, 0, 2, 0), dr: 1, dc: 0 };
        assert_eq!(m.reescrever("=A2+A4", "P"), "=A3+#REF!");
    }

    #[test]
    fn nada_muda_nada_se_reescreve() {
        let m = mov();
        // Espaço e caixa preservados quando nenhuma referência é do bloco.
        assert_eq!(m.reescrever("=soma( C1 ;  2,5 )", "Plan1"), "=soma( C1 ;  2,5 )");
        assert_eq!(m.reescrever("texto A1", "Plan1"), "texto A1");
    }

    #[test]
    fn destino_fora_da_planilha() {
        let m = Movimento { aba: "P".into(), origem: Rect::new(0, 0, 0, 0), dr: -1, dc: 0 };
        assert!(m.destino().is_none());
        assert_eq!(m.reescrever("=A1", "P"), "=A1");
    }
}
