//! O bloco — um pedaço retangular da planilha, em binário, para o TypeScript
//! guardar num cache pequeno e ler célula a célula sem voltar à ponte.
//!
//! É o parente do [`crate::planilha::frame`], com uma diferença de propósito: o quadro
//! leva só o que o desenho precisa (texto pronto e bandeiras), e o bloco leva
//! a célula **inteira** — o valor tipado, o texto digitado, o texto
//! formatado e o índice do estilo direto. É o que deixa o TypeScript
//! continuar respondendo `getValue`, `getRaw` e `getCell` de forma síncrona,
//! como a interface inteira espera, sem ser dono de célula nenhuma: ele tem
//! uns poucos blocos, jogados fora a cada versão nova da aba.
//!
//! ## O formato
//!
//! Tudo *little-endian*. O cabeçalho tem 32 bytes para os `f64` que vêm logo
//! depois começarem alinhados em 8 — sem isso o JavaScript não monta um
//! `Float64Array` por cima e precisaria copiar.
//!
//! ```text
//! 0   "BLC1"
//! 4   versão da aba                u32
//! 8   r0, c0, r1, c1               u32 x4
//! 24  quantidade de células (n)    u32
//! 28  quantos textos por célula (k) u32   (4 com texto, 2 sem)
//! 32  número[]                     f64 x n
//!     linha[]                      u32 x n
//!     coluna[]                     u32 x n
//!     estilo[]                     u32 x n   índice no dicionário da aba,
//!                                            SEM_ESTILO = nenhum
//!     início[]                     u32 x k·n  (em bytes, no bloco de texto)
//!     tamanho[]                    u32 x k·n  (em BYTES)
//!     tipo[]                       u8  x n
//!     texto                        UTF-8 emendado
//! ```
//!
//! Os `k` textos de cada célula, em ordem: o **texto do valor** (texto da
//! célula, ou o código do erro), o **detalhe** do erro, o **texto digitado**
//! e o **texto formatado**. Os dois últimos só vêm quando pedidos: quem soma
//! uma coluna de dois milhões de linhas quer o valor, não a formatação.

use crate::celulas::estilo_parcial::para_style;
use crate::celulas::refs::{key, key_col, key_row, CellKey};
use crate::celulas::style::format_number;
use crate::celulas::values::Value;
use crate::planilha::workbook::Workbook;

pub const MAGIC: [u8; 4] = *b"BLC1";
pub const SEM_ESTILO: u32 = u32::MAX;

pub const TIPO_VAZIO: u8 = 0;
pub const TIPO_NUMERO: u8 = 1;
pub const TIPO_TEXTO: u8 = 2;
pub const TIPO_LOGICO: u8 = 3;
pub const TIPO_ERRO: u8 = 4;
/// Somado ao tipo quando a célula é fórmula.
pub const BANDEIRA_FORMULA: u8 = 0x10;

/// Acima disto o retângulo não é varrido posição a posição: pergunta-se à
/// aba o que existe nele.
const VARRER_ATE: u64 = 65_536;

/// O `formatValue` do `style.ts`, para o valor já calculado.
pub fn texto_formatado(valor: &Value, estilo: Option<&crate::celulas::style::Style>) -> String {
    // Formato personalizado (`;;;`, `0" kWp"`) manda sobre o resto.
    if let Some(codigo) = estilo.and_then(|e| e.code.as_deref()) {
        if let Some(t) = crate::celulas::formato_personalizado::formatar(valor, codigo) { return t; }
    }
    match valor {
        Value::Empty => String::new(),
        Value::Error(e) => e.code.as_str().to_string(),
        Value::Number(n) => format_number(*n, estilo),
        Value::Text(t) => t.clone(),
        Value::Bool(b) => if *b { "VERDADEIRO" } else { "FALSO" }.to_string(),
    }
}

struct Celula {
    row: u32,
    col: u32,
    tipo: u8,
    numero: f64,
    estilo: u32,
    textos: [String; 4],
}

/// Monta o bloco do retângulo `r0..=r1` × `c0..=c1` da aba `indice`.
///
/// Entra toda célula com conteúdo ou com estilo direto. `com_texto` decide
/// se vão o texto digitado e o formatado.
pub fn montar(pasta: &Workbook, indice: usize, r0: u32, c0: u32, r1: u32, c1: u32, com_texto: bool) -> Vec<u8> {
    let Some(aba) = pasta.aba(indice) else {
        return cabecalho(0, r0, c0, r1, c1, 0, if com_texto { 4 } else { 2 });
    };
    let sheet = &aba.sheet;
    let usado = sheet.used_rect();
    let (lr0, lc0) = (r0.max(usado.r0), c0.max(usado.c0));
    let (lr1, lc1) = (r1.min(usado.r1), c1.min(usado.c1));

    let mut chaves: Vec<CellKey> = Vec::new();
    if lr0 <= lr1 && lc0 <= lc1 && !sheet.is_empty_de_tudo() {
        let area = (lr1 - lr0 + 1) as u64 * (lc1 - lc0 + 1) as u64;
        if area <= VARRER_ATE {
            for row in lr0..=lr1 {
                for col in lc0..=lc1 {
                    chaves.push(key(row, col));
                }
            }
        } else {
            chaves = sheet.chaves_no_retangulo(&crate::celulas::refs::Rect::new(lr0, lc0, lr1, lc1));
        }
    }

    let mut celulas = Vec::new();
    for k in chaves {
        let (row, col) = (key_row(k), key_col(k));
        let estilo = sheet.get_style_idx(row, col);
        let tem = sheet.has_content(row, col);
        if !tem && estilo.is_none() {
            continue;
        }
        let valor = if tem { pasta.valor(indice, row, col) } else { Value::Empty };
        let raw = if com_texto { sheet.get_raw(row, col) } else { String::new() };
        let formula = raw.starts_with('=') || (!com_texto && sheet.is_formula(row, col));
        let (mut tipo, numero, t_valor, detalhe) = match &valor {
            Value::Empty => (TIPO_VAZIO, 0.0, String::new(), String::new()),
            Value::Number(n) => (TIPO_NUMERO, *n, String::new(), String::new()),
            Value::Text(t) => (TIPO_TEXTO, 0.0, t.clone(), String::new()),
            Value::Bool(b) => (TIPO_LOGICO, if *b { 1.0 } else { 0.0 }, String::new(), String::new()),
            Value::Error(e) => (
                TIPO_ERRO,
                0.0,
                e.code.as_str().to_string(),
                e.detail.clone().unwrap_or_default(),
            ),
        };
        if formula {
            tipo |= BANDEIRA_FORMULA;
        }
        let formatado = if com_texto {
            let resolvido = sheet.estilo_mesclado(row, col).map(|e| para_style(&e));
            texto_formatado(&valor, resolvido.as_ref())
        } else {
            String::new()
        };
        celulas.push(Celula {
            row,
            col,
            tipo,
            numero,
            estilo: estilo.map(|i| i as u32).unwrap_or(SEM_ESTILO),
            textos: [t_valor, detalhe, raw, formatado],
        });
    }

    let k = if com_texto { 4 } else { 2 };
    let n = celulas.len();
    let mut out = cabecalho(aba.versao, r0, c0, r1, c1, n as u32, k as u32);
    out.reserve(n * (8 + 12 + 8 * k + 1) + 16);
    for c in &celulas {
        out.extend_from_slice(&c.numero.to_le_bytes());
    }
    for c in &celulas {
        out.extend_from_slice(&c.row.to_le_bytes());
    }
    for c in &celulas {
        out.extend_from_slice(&c.col.to_le_bytes());
    }
    for c in &celulas {
        out.extend_from_slice(&c.estilo.to_le_bytes());
    }
    let mut blob: Vec<u8> = Vec::new();
    let mut inicios = Vec::with_capacity(n * k);
    let mut tamanhos = Vec::with_capacity(n * k);
    for c in &celulas {
        for t in c.textos.iter().take(k) {
            inicios.push(blob.len() as u32);
            tamanhos.push(t.len() as u32);
            blob.extend_from_slice(t.as_bytes());
        }
    }
    for i in inicios {
        out.extend_from_slice(&i.to_le_bytes());
    }
    for t in tamanhos {
        out.extend_from_slice(&t.to_le_bytes());
    }
    for c in &celulas {
        out.push(c.tipo);
    }
    out.extend_from_slice(&blob);
    out
}

fn cabecalho(versao: u32, r0: u32, c0: u32, r1: u32, c1: u32, n: u32, k: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(&MAGIC);
    for v in [versao, r0, c0, r1, c1, n, k] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Todas as células da aba com o texto digitado e o estilo direto — o
/// `forEachCell` do TypeScript, para quem precisa da aba inteira (mover
/// linha, gravar o JSON, duplicar a aba).
///
/// ```text
/// 0  "CEL1"
/// 4  n                  u32
/// 8  linha[]            u32 x n
///    coluna[]           u32 x n
///    estilo[]           u32 x n
///    início[]           u32 x n
///    tamanho[]          u32 x n
///    texto              UTF-8 emendado
/// ```
pub fn montar_celulas(pasta: &Workbook, indice: usize) -> Vec<u8> {
    let mut linhas = Vec::new();
    let mut colunas = Vec::new();
    let mut estilos = Vec::new();
    let mut textos: Vec<String> = Vec::new();
    if let Some(aba) = pasta.aba(indice) {
        let sheet = &aba.sheet;
        sheet.for_each_snapshot(&mut |row, col, snap| {
            linhas.push(row);
            colunas.push(col);
            estilos.push(sheet.get_style_idx(row, col).map(|i| i as u32).unwrap_or(SEM_ESTILO));
            textos.push(snap.raw.clone());
        });
    }
    let n = linhas.len();
    let mut out = Vec::with_capacity(8 + n * 20);
    out.extend_from_slice(b"CEL1");
    out.extend_from_slice(&(n as u32).to_le_bytes());
    for v in linhas.iter().chain(&colunas).chain(&estilos) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    let mut pos = 0u32;
    for t in &textos {
        out.extend_from_slice(&pos.to_le_bytes());
        pos += t.len() as u32;
    }
    for t in &textos {
        out.extend_from_slice(&(t.len() as u32).to_le_bytes());
    }
    for t in &textos {
        out.extend_from_slice(t.as_bytes());
    }
    out
}

#[cfg(test)]
mod testes {
    use super::*;

    fn u32_em(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
    }

    #[test]
    fn o_bloco_leva_valor_texto_e_formatacao() {
        let mut w = Workbook::new();
        w.editar(0, |s| {
            s.set_cell(0, 0, "10%");
            s.set_cell(0, 1, "=A1*2");
            s.set_cell(1, 0, "olá");
            s.set_cell(1, 1, "=1/0");
        });
        let b = montar(&w, 0, 0, 0, 9, 9, true);
        assert_eq!(&b[0..4], b"BLC1");
        let n = u32_em(&b, 24) as usize;
        assert_eq!(n, 4);
        assert_eq!(u32_em(&b, 28), 4);
        assert_eq!(32 % 8, 0, "os f64 precisam começar alinhados");
        let num = |i: usize| f64::from_le_bytes(b[32 + 8 * i..40 + 8 * i].try_into().unwrap());
        assert_eq!(num(0), 0.1);
        assert!((num(1) - 0.2).abs() < 1e-12);
        let base_u32 = 32 + 8 * n;
        let tipos = base_u32 + 4 * n * 3 + 4 * n * 4 * 2;
        assert_eq!(b[tipos], TIPO_NUMERO);
        assert_eq!(b[tipos + 1], TIPO_NUMERO | BANDEIRA_FORMULA);
        assert_eq!(b[tipos + 2], TIPO_TEXTO);
        assert_eq!(b[tipos + 3], TIPO_ERRO | BANDEIRA_FORMULA);
    }

    #[test]
    fn bloco_de_versao_sem_celulas_so_tem_cabecalho() {
        let w = Workbook::new();
        let b = montar(&w, 0, 0, 0, 100, 100, true);
        assert_eq!(b.len(), 32);
    }
}
