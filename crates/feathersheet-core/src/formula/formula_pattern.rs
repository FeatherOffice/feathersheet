//! Padrão de fórmula: o que `=B2+C2`, `=B3+C3` e `=B4+C4` têm em comum.
//!
//! Tradução de `src/core/formula-pattern.ts`.
//!
//! Uma coluna preenchida para baixo não tem um milhão de fórmulas — tem
//! *uma*, repetida com a linha deslocando. Guardar o texto de cada célula
//! custa ~80 bytes por célula e a árvore custa ~750; guardar o padrão uma
//! vez e, por célula, só o número dele custa 4.
//!
//! O padrão é o texto da fórmula com cada referência trocada por um
//! marcador que diz **de onde ela fala**: deslocamento em relação à própria
//! célula quando é relativa, posição fixa quando tem cifrão. Duas células
//! têm o mesmo padrão quando esse texto bate.
//!
//! O caminho é textual, como o `map_references` de que ele vive
//! ([`crate::formula::formula_text`]): o que o analisador não entende passa intacto
//! e texto entre aspas nunca é tocado. A garantia de que nada se perde é o
//! [`pattern_for`], que só aceita um padrão depois de conferir que ele
//! reconstrói o texto original de volta.
//!
//! É esta peça que destrava a limitação V1 do `sheet-io::fshz_write`
//! (coluna de fórmula nunca "derrama" para avulsa — ver o comentário de
//! topo daquele arquivo): sem um jeito de reconstruir o texto de uma
//! fórmula a partir do padrão, uma coluna de fórmula esparsa não tinha como
//! virar célula avulsa.

use crate::formula::formula_text::{format_ref, map_references, ParsedRef};

/// Delimita um marcador de referência dentro do texto do padrão. É um
/// caractere de controle: não aparece em fórmula escrita por gente, e por
/// isso não há como confundi-lo com o conteúdo.
const MARK: char = '\u{1}';
const MARK_BYTE: u8 = 1;

/// Monta o marcador de uma referência, visto da célula (`row`, `col`).
fn encode_ref(r: &ParsedRef, row: i32, col: i32) -> String {
    // Com cifrão a referência não anda: guarda a posição absoluta. Sem
    // cifrão, guarda a distância — é o que faz B2 (visto de D2) e B3 (visto
    // de D3) virarem o mesmo "duas colunas à esquerda, mesma linha".
    let c = if r.col_lock { format!("a{}", r.col) } else { format!("r{}", r.col - col) };
    let rr = if r.row_lock { format!("a{}", r.row) } else { format!("r{}", r.row - row) };
    let sheet = match &r.sheet {
        Some(s) => format!(":{s}"),
        None => String::new(),
    };
    format!("{MARK}{c};{rr}{sheet}{MARK}")
}

/// Lê o marcador de volta para uma referência, vista da célula dada.
fn decode_ref(body: &str, row: i32, col: i32) -> ParsedRef {
    let sep = body.find(';').unwrap_or(body.len());
    let c_part = &body[..sep];
    let rest = if sep < body.len() { &body[sep + 1..] } else { "" };
    // O nome da aba vem por último e pode conter qualquer coisa, então é o
    // resto inteiro depois do primeiro ':'.
    let sheet_sep = rest.find(':');
    let (r_part, sheet) = match sheet_sep {
        Some(idx) => (&rest[..idx], Some(rest[idx + 1..].to_string())),
        None => (rest, None),
    };
    let col_lock = c_part.as_bytes().first() == Some(&b'a');
    let row_lock = r_part.as_bytes().first() == Some(&b'a');
    let cv: i32 = c_part.get(1..).and_then(|s| s.parse().ok()).unwrap_or(0);
    let rv: i32 = r_part.get(1..).and_then(|s| s.parse().ok()).unwrap_or(0);
    ParsedRef {
        sheet,
        col_lock,
        row_lock,
        col: if col_lock { cv } else { col + cv },
        row: if row_lock { rv } else { row + rv },
    }
}

/// O texto do padrão da fórmula, visto da célula (`row`, `col`).
pub fn pattern_of(raw: &str, row: i32, col: i32) -> String {
    map_references(raw, |r| encode_ref(&r, row, col))
}

/// O texto da fórmula que o padrão produz na célula (`row`, `col`).
pub fn expand_pattern(pattern: &str, row: i32, col: i32) -> String {
    if !pattern.contains(MARK) {
        return pattern.to_string();
    }
    let mut out = String::new();
    let mut i = 0usize;
    let bytes = pattern.as_bytes();
    while i < pattern.len() {
        if bytes[i] != MARK_BYTE {
            let ch = pattern[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        match pattern[i + 1..].find(MARK) {
            Some(rel_end) => {
                let end = i + 1 + rel_end;
                let body = &pattern[i + 1..end];
                out.push_str(&format_ref(&decode_ref(body, row, col)));
                i = end + 1;
            }
            None => {
                out.push_str(&pattern[i..]);
                break;
            }
        }
    }
    out
}

/// Um pedaço de um padrão já separado: o texto que não muda de célula para
/// célula, ou uma referência lida na célula (0, 0) — a posição fixa quando
/// tem cifrão, o deslocamento quando não tem ([`ref_em`] a põe no lugar).
///
/// Serve quem expande o mesmo padrão milhões de vezes (o exportador de
/// `.xlsx`): separar uma vez poupa reler o marcador em cada célula.
/// Juntar os pedaços com [`format_ref`] dá o mesmo que o [`expand_pattern`].
#[derive(Debug, Clone, PartialEq)]
pub enum Pedaco {
    Texto(String),
    Ref(ParsedRef),
}

/// O padrão em pedaços. Um marcador sem fim vira texto, como no
/// [`expand_pattern`].
pub fn pedacos(pattern: &str) -> Vec<Pedaco> {
    let mut out = Vec::new();
    let mut texto = String::new();
    let mut resto = pattern;
    while let Some(i) = resto.find(MARK) {
        let Some(fim) = resto[i + 1..].find(MARK) else { break };
        texto.push_str(&resto[..i]);
        if !texto.is_empty() {
            out.push(Pedaco::Texto(std::mem::take(&mut texto)));
        }
        out.push(Pedaco::Ref(decode_ref(&resto[i + 1..i + 1 + fim], 0, 0)));
        resto = &resto[i + 1 + fim + 1..];
    }
    texto.push_str(resto);
    if !texto.is_empty() {
        out.push(Pedaco::Texto(texto));
    }
    out
}

/// A referência de um [`Pedaco::Ref`] vista da célula (`row`, `col`).
pub fn ref_em(r: &ParsedRef, row: i32, col: i32) -> ParsedRef {
    ParsedRef {
        sheet: r.sheet.clone(),
        col_lock: r.col_lock,
        row_lock: r.row_lock,
        col: if r.col_lock { r.col } else { col + r.col },
        row: if r.row_lock { r.row } else { row + r.row },
    }
}

/// O padrão da fórmula, ou `None` quando ela não pode virar padrão.
///
/// A recusa é sempre por segurança: só devolve um padrão depois de
/// conferir que ele reconstrói **exatamente** o texto que entrou. Fórmula
/// com algo que o `map_references` reescreve de forma diferente (um
/// `#REF!`, uma sintaxe que ele não reconhece) simplesmente não vira
/// padrão e segue pelo caminho antigo — o mesmo espírito do `roundTrips`
/// no `sheet.ts`.
pub fn pattern_for(raw: &str, row: i32, col: i32) -> Option<String> {
    if !raw.starts_with('=') || raw.chars().count() < 2 {
        return None;
    }
    let pattern = pattern_of(raw, row, col);
    if expand_pattern(&pattern, row, col) != raw {
        return None;
    }
    Some(pattern)
}

/// Uma referência a célula única que um padrão lê, como deslocamento.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternRef {
    /// Deslocamento de linha; `row` absoluto quando `row_lock`.
    pub row: i32,
    pub col: i32,
    pub row_lock: bool,
    pub col_lock: bool,
    /// Referência qualificada: conservada por padrão para invalidação pela
    /// pasta e reescrita em edições estruturais.
    pub sheet: Option<String>,
}

/// As referências de célula única que o padrão lê.
///
/// É o que permite **deduzir** as dependências em vez de guardá-las: se o
/// padrão da coluna lê "mesma linha, duas colunas à esquerda", então mexer
/// em B7 suja D7 — e isso se calcula na hora, sem um `Set` por célula.
///
/// Intervalos locais ficam de fora: são de `faixas_do_padrao`. Intervalos
/// qualificados conservam as duas pontas e o nome da aba: a pasta invalida
/// esses padrões após uma edição, sem registrar uma dependência por célula.
pub fn refs_of_pattern(pattern: &str) -> Option<Vec<PatternRef>> {
    let mut refs = Vec::new();
    let mut i = 0usize;
    let bytes = pattern.as_bytes();
    while i < pattern.len() {
        if bytes[i] != MARK_BYTE {
            i += 1;
            continue;
        }
        let end = match pattern[i + 1..].find(MARK) {
            Some(rel_end) => i + 1 + rel_end,
            None => break,
        };
        let r = decode_ref(&pattern[i + 1..end], 0, 0);
        // A segunda ponta de `Produtos!A1:A9` herda a aba da primeira.
        if pattern.as_bytes().get(end + 1) == Some(&b':') && pattern.as_bytes().get(end + 2) == Some(&MARK_BYTE) {
            let start2 = end + 3;
            let end2 = start2 + pattern[start2..].find(MARK)?;
            // Intervalo local (`A2:A10`): fica com `faixas_do_padrao`
            // (`modelo_padrao.rs`), que sabe a área inteira, não só as pontas.
            let Some(sheet) = r.sheet.clone() else {
                i = end2 + 1;
                continue;
            };
            let mut r2 = decode_ref(&pattern[start2..end2], 0, 0);
            r2.sheet = Some(sheet);
            refs.push(PatternRef { row: r.row, col: r.col, row_lock: r.row_lock, col_lock: r.col_lock, sheet: r.sheet });
            refs.push(PatternRef { row: r2.row, col: r2.col, row_lock: r2.row_lock, col_lock: r2.col_lock, sheet: r2.sheet });
            i = end2 + 1;
            continue;
        }
        refs.push(PatternRef { row: r.row, col: r.col, row_lock: r.row_lock, col_lock: r.col_lock, sheet: r.sheet });
        i = end + 1;
    }
    Some(refs)
}

#[cfg(test)]
mod testes_pedacos {
    use super::*;

    #[test]
    fn juntar_os_pedacos_da_o_mesmo_que_expandir() {
        for (raw, row, col) in [
            ("=A1+$B$2*'Meu Plano'!C3", 0, 3),
            ("=SOMA(A1:A10;\"A1\")", 9, 1),
            ("=HOJE()", 4, 4),
            ("=A1", 0, 1),
            ("=Dados!$A7&B$1", 6, 2),
        ] {
            let padrao = pattern_of(raw, row, col);
            for (r, c) in [(row, col), (row + 5, col), (0, 0), (100, col + 2)] {
                let juntado: String = pedacos(&padrao)
                    .iter()
                    .map(|p| match p {
                        Pedaco::Texto(t) => t.clone(),
                        Pedaco::Ref(x) => format_ref(&ref_em(x, r, c)),
                    })
                    .collect();
                assert_eq!(juntado, expand_pattern(&padrao, r, c), "{raw} em ({r}, {c})");
            }
        }
        // Marcador sem fim vira texto.
        assert_eq!(pedacos("=A\u{1}r0"), vec![Pedaco::Texto("=A\u{1}r0".into())]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celulas_com_a_mesma_formula_relativa_tem_o_mesmo_padrao() {
        let p2 = pattern_for("=B2+C2", 1, 3).unwrap(); // D2
        let p3 = pattern_for("=B3+C3", 2, 3).unwrap(); // D3
        let p4 = pattern_for("=B4+C4", 3, 3).unwrap(); // D4
        assert_eq!(p2, p3);
        assert_eq!(p3, p4);
    }

    #[test]
    fn expandir_o_padrao_reconstroi_a_formula_de_cada_linha() {
        let pattern = pattern_for("=B2+C2", 1, 3).unwrap();
        assert_eq!(expand_pattern(&pattern, 1, 3), "=B2+C2");
        assert_eq!(expand_pattern(&pattern, 2, 3), "=B3+C3");
        assert_eq!(expand_pattern(&pattern, 99, 3), "=B100+C100");
    }

    #[test]
    fn referencia_absoluta_nao_anda_com_a_linha() {
        let pattern = pattern_for("=$B$2+C2", 1, 3).unwrap();
        assert_eq!(expand_pattern(&pattern, 1, 3), "=$B$2+C2");
        // A absoluta fica parada; a relativa acompanha a linha.
        assert_eq!(expand_pattern(&pattern, 5, 3), "=$B$2+C6");
    }

    #[test]
    fn referencia_para_outra_aba_preserva_o_nome() {
        let pattern = pattern_for("=Dados!A1+B1", 0, 5).unwrap();
        assert_eq!(expand_pattern(&pattern, 0, 5), "=Dados!A1+B1");
        // Referência para outra aba também é relativa (sem cifrão) — anda
        // junto com a linha igual a uma da mesma aba; só o nome da aba fica.
        assert_eq!(expand_pattern(&pattern, 10, 5), "=Dados!A11+B11");
    }

    #[test]
    fn nome_de_aba_com_aspa_simples_sobrevive_a_ida_e_volta() {
        let raw = "='Meu Plano'!A1+B1";
        let pattern = pattern_for(raw, 0, 5).unwrap();
        assert_eq!(expand_pattern(&pattern, 0, 5), raw);
    }

    #[test]
    fn texto_entre_aspas_no_meio_da_formula_nao_e_tocado() {
        let raw = "=A1&\" e B2 \"&B1";
        let pattern = pattern_for(raw, 0, 0).unwrap();
        assert_eq!(expand_pattern(&pattern, 0, 0), raw);
        // "B2" dentro da string não virou marcador — só A1 e B1.
        assert_eq!(refs_of_pattern(&pattern).unwrap().len(), 2);
    }

    #[test]
    fn quem_nao_e_formula_nao_vira_padrao() {
        assert_eq!(pattern_for("10", 0, 0), None);
        assert_eq!(pattern_for("=", 0, 0), None);
        assert_eq!(pattern_for("texto qualquer", 0, 0), None);
    }

    #[test]
    fn intervalo_local_fica_fora_das_refs() {
        // Desde 17/10/2026 o intervalo local é de `faixas_do_padrao`.
        let pattern = pattern_for("=SOMA(A1:A9)+B1", 0, 2).unwrap();
        let refs = refs_of_pattern(&pattern).unwrap();
        assert_eq!(refs.len(), 1, "só B1");
    }

    #[test]
    fn refs_of_pattern_devolve_deslocamento_de_cada_referencia() {
        let pattern = pattern_for("=B2+$C$2", 1, 3).unwrap(); // D2: B relativa, C absoluta
        let refs = refs_of_pattern(&pattern).unwrap();
        assert_eq!(refs.len(), 2);
        assert!(!refs[0].col_lock);
        assert!(!refs[0].row_lock);
        assert_eq!(refs[0].col, -2); // B (1) - D (3) = -2
        assert_eq!(refs[0].row, 0); // mesma linha
        assert!(refs[1].col_lock);
        assert_eq!(refs[1].col, 2); // C absoluto = coluna 2
        assert_eq!(refs[1].row, 1); // linha 2 (índice 1) absoluta
    }

    #[test]
    fn formula_com_se_e_comparacao_vira_padrao() {
        let raw1 = "=SE(D2>200;D2*1.15;D2*1.05)";
        let p1 = pattern_for(raw1, 1, 4);
        assert!(p1.is_some(), "p1 failed");

        let raw2 = "=SE(E2>300;\"ALTO\";\"NORMAL\")";
        let p2 = pattern_for(raw2, 1, 5);
        assert!(p2.is_some(), "p2 failed for {}", raw2);
    }

    #[test]
    fn mesma_formula_em_colunas_distintas_tem_padroes_diferentes() {
        assert_ne!(
            pattern_for("=B2+C2", 1, 3),
            pattern_for("=B2+C2", 1, 4)
        );
    }

    #[test]
    fn padrao_sem_referencia_e_referencia_absoluta_nao_se_deslocam() {
        let fixo = pattern_for("=$A$1*2", 1, 3).unwrap();
        assert_eq!(expand_pattern(&fixo, 50, 7), "=$A$1*2");
        let literal = pattern_for("=1+1", 0, 0).unwrap();
        assert_eq!(expand_pattern(&literal, 500, 3), "=1+1");
    }

    #[test]
    fn padrao_preserva_exatamente_o_texto_de_formulas_especiais() {
        for raw in [
            "=A1+#REF!",
            "=SOMA(A1:A9)",
            "=B2&\"x\"",
            "=-A1",
            "=A1<=B1",
        ] {
            if let Some(padrao) = pattern_for(raw, 5, 5) {
                assert_eq!(expand_pattern(&padrao, 5, 5), raw);
            }
        }
    }
}
