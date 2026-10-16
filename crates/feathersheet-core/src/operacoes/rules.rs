//! Formatação condicional — tradução do `rules.ts`.
//!
//! As regras vivem por aba e são consultadas na hora de desenhar: nada é
//! gravado no estilo da célula, senão pintar uma coluna inteira estouraria o
//! arquivo. A grade pede o que pintar no retângulo visível, uma vez por
//! quadro (`condicional_bloco` na ponte).
//!
//! O `rules.rs` anterior (23/09/2026) não era usado por ninguém e divergia da
//! referência: a regra de fórmula personalizada nunca pintava (as "Cores
//! alternadas" do menu), e o valor da regra era lido com o `parse` do Rust em
//! vez do `parseLiteral` — "1,5" e data digitada não comparavam.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::formula::ast::Node;
use crate::operacoes::fill::shift_formula;
use crate::celulas::input::parse_literal;
use crate::formula::parser::parse_formula;
use crate::celulas::values::{compare, to_boolean, to_number, to_text, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CellOp {
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
    Equal,
    NotEqual,
    Between,
    NotBetween,
    IsEmpty,
    IsNotEmpty,
    TextContains,
    TextNotContains,
    TextStartsWith,
    TextEndsWith,
    DateBefore,
    DateAfter,
    DateOn,
    Duplicate,
    Unique,
    AboveAverage,
    BelowAverage,
    Formula,
}

/// O que a regra pinta quando bate. Ausente e `false` são diferentes: a
/// segunda regra só preenche o que a primeira deixou em aberto.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CondStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strike: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellCondition {
    pub op: CellOp,
    /// Texto cru dos campos, como o usuário digitou ("10", "01/01/2026", "=A1>B1").
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub style: CondStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScalePointType {
    Min,
    Max,
    Number,
    Percent,
    Percentile,
}

/// Ponta de uma escala: um número, um percentual, um percentil ou o extremo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalePoint {
    #[serde(rename = "type")]
    pub point_type: ScalePointType,
    #[serde(default)]
    pub value: Option<f64>,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorScaleCondition {
    pub min: ScalePoint,
    #[serde(default)]
    pub mid: Option<ScalePoint>,
    pub max: ScalePoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataBarCondition {
    pub color: String,
    /// Cor das barras negativas; ausente usa a mesma cor.
    #[serde(default)]
    pub negative_color: Option<String>,
    /// Pinta só a barra, sem o número.
    #[serde(default)]
    pub hide_value: bool,
    #[serde(default)]
    pub min: Option<ScalePoint>,
    #[serde(default)]
    pub max: Option<ScalePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Condition {
    #[serde(rename = "cell")]
    Cell(CellCondition),
    #[serde(rename = "colorScale")]
    ColorScale(ColorScaleCondition),
    #[serde(rename = "dataBar")]
    DataBar(DataBarCondition),
}

/// O retângulo como o arquivo guarda (`{ r0, c0, r1, c1 }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faixa {
    pub r0: u32,
    pub c0: u32,
    pub r1: u32,
    pub c1: u32,
}

impl Faixa {
    fn contem(&self, row: u32, col: u32) -> bool {
        row >= self.r0 && row <= self.r1 && col >= self.c0 && col <= self.c1
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalRule {
    pub id: String,
    pub ranges: Vec<Faixa>,
    pub condition: Condition,
}

/// Barra de dados já resolvida em frações da largura da célula.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarPaint {
    pub from: f64,
    pub to: f64,
    pub color: String,
    pub hide_value: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct CellFormat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<CondStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bar: Option<BarPaint>,
}

/// O que o motor de regras precisa da aba (o `RuleHost` do TypeScript).
pub trait RuleHost {
    fn valor(&self, row: u32, col: u32) -> Value;
    /// Avalia uma árvore avulsa como se ela estivesse na célula dada.
    fn avaliar_em(&self, row: u32, col: u32, node: &Node) -> Value;
    /// Última linha e coluna ocupadas, para não varrer o vazio.
    fn max_row(&self) -> u32;
    fn max_col(&self) -> u32;
}

#[derive(Debug, Clone)]
struct Stats {
    min: f64,
    max: f64,
    avg: f64,
    /// Os números do intervalo em ordem, para os percentis.
    sorted: Vec<f64>,
}

/// Guarda as regras de uma aba e responde o que pintar numa célula. As
/// estatísticas de intervalo (mínimo, máximo, repetidos) ficam em cache;
/// quem guarda o objeto as joga fora quando a aba muda.
#[derive(Debug, Clone, Default)]
pub struct ConditionalFormats {
    pub rules: Vec<ConditionalRule>,
    stats: HashMap<String, Stats>,
    /// Quantas vezes cada valor aparece; só montado por repetido/único.
    counts: HashMap<String, HashMap<String, usize>>,
    asts: HashMap<String, Option<Node>>,
}

impl ConditionalFormats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn set_rules(&mut self, rules: Vec<ConditionalRule>) {
        self.rules = rules;
        self.stats.clear();
        self.counts.clear();
        self.asts.clear();
    }

    /// Joga fora todas as estatísticas.
    pub fn invalidate(&mut self) {
        self.stats.clear();
        self.counts.clear();
    }

    fn covers(rule: &ConditionalRule, row: u32, col: u32) -> bool {
        rule.ranges.iter().any(|r| r.contem(row, col))
    }

    fn range_of(rule: &ConditionalRule, row: u32, col: u32) -> Faixa {
        rule.ranges.iter().find(|r| r.contem(row, col)).copied().unwrap_or(rule.ranges[0])
    }

    /// O que pintar nesta célula. A primeira regra que bate manda em cada
    /// propriedade — é como o Google Planilhas resolve o conflito.
    pub fn format_for(&mut self, host: &dyn RuleHost, row: u32, col: u32, value: &Value) -> Option<CellFormat> {
        if self.rules.is_empty() {
            return None;
        }
        let mut style: Option<CondStyle> = None;
        let mut bar: Option<BarPaint> = None;
        for i in 0..self.rules.len() {
            if !Self::covers(&self.rules[i], row, col) {
                continue;
            }
            let rule = self.rules[i].clone();
            match &rule.condition {
                Condition::Cell(cond) => {
                    if !self.test_cell(host, &rule, cond, row, col, value) {
                        continue;
                    }
                    style = Some(match style {
                        Some(s) => fill_gaps(s, &cond.style),
                        None => cond.style.clone(),
                    });
                }
                Condition::ColorScale(cond) => {
                    if style.as_ref().is_some_and(|s| s.bg.is_some()) {
                        continue;
                    }
                    if let Some(bg) = self.scale_color(host, &rule, cond, value) {
                        let patch = CondStyle { bg: Some(bg), ..Default::default() };
                        style = Some(fill_gaps(style.unwrap_or_default(), &patch));
                    }
                }
                Condition::DataBar(cond) => {
                    if bar.is_none() {
                        bar = self.bar_for(host, &rule, cond, value);
                    }
                }
            }
        }
        if style.is_none() && bar.is_none() {
            None
        } else {
            Some(CellFormat { style, bar })
        }
    }

    // ---------------------------------------------------------- condições

    fn test_cell(&mut self, host: &dyn RuleHost, rule: &ConditionalRule, cond: &CellCondition, row: u32, col: u32, value: &Value) -> bool {
        let op = cond.op;
        let raw = &cond.values;
        let vazio = matches!(value, Value::Empty) || matches!(value, Value::Text(t) if t.is_empty());

        if op == CellOp::IsEmpty {
            return vazio;
        }
        if op == CellOp::IsNotEmpty {
            return !vazio;
        }
        if op == CellOp::Formula {
            return self.test_formula(host, rule, raw.first().map(String::as_str).unwrap_or(""), row, col);
        }
        // Célula vazia não entra nas demais comparações (é o que o Sheets faz).
        if vazio || matches!(value, Value::Error(_)) {
            return false;
        }

        if op == CellOp::Duplicate || op == CellOp::Unique {
            let n = self.counts_of(host, rule).get(&bucket_key(value)).copied().unwrap_or(0);
            return if op == CellOp::Duplicate { n > 1 } else { n == 1 };
        }

        if op == CellOp::AboveAverage || op == CellOp::BelowAverage {
            let Some(n) = numeric(value) else { return false };
            let avg = self.stats_of(host, rule).avg;
            return if op == CellOp::AboveAverage { n > avg } else { n < avg };
        }

        if matches!(op, CellOp::TextContains | CellOp::TextNotContains | CellOp::TextStartsWith | CellOp::TextEndsWith) {
            let needle = raw.first().map(|s| s.to_lowercase()).unwrap_or_default();
            if needle.is_empty() {
                return false;
            }
            let hay = to_text(value).to_lowercase();
            let hit = match op {
                CellOp::TextStartsWith => hay.starts_with(&needle),
                CellOp::TextEndsWith => hay.ends_with(&needle),
                _ => hay.contains(&needle),
            };
            return if op == CellOp::TextNotContains { !hit } else { hit };
        }

        let a = literal(raw.first());
        if op == CellOp::Between || op == CellOp::NotBetween {
            let b = literal(raw.get(1));
            let (Ok(lo), Ok(hi)) = (compare(value, &a), compare(value, &b)) else { return false };
            let inside = lo.is_ge() && hi.is_le();
            return if op == CellOp::Between { inside } else { !inside };
        }

        let Ok(c) = compare(value, &a) else { return false };
        match op {
            CellOp::GreaterThan | CellOp::DateAfter => c.is_gt(),
            CellOp::GreaterOrEqual => c.is_ge(),
            CellOp::LessThan | CellOp::DateBefore => c.is_lt(),
            CellOp::LessOrEqual => c.is_le(),
            CellOp::Equal | CellOp::DateOn => c.is_eq(),
            CellOp::NotEqual => c.is_ne(),
            _ => false,
        }
    }

    /// A fórmula é escrita para o canto superior esquerdo do intervalo; para
    /// as demais células as referências relativas escorregam junto, como no
    /// Excel. A árvore deslocada fica em cache por deslocamento.
    fn test_formula(&mut self, host: &dyn RuleHost, rule: &ConditionalRule, text: &str, row: u32, col: u32) -> bool {
        let body = text.strip_prefix('=').unwrap_or(text);
        if body.trim().is_empty() {
            return false;
        }
        let rect = Self::range_of(rule, row, col);
        let (dr, dc) = (row as i32 - rect.r0 as i32, col as i32 - rect.c0 as i32);
        let chave = format!("{}:{dr},{dc}", rule.id);
        let ast = self.asts.entry(chave).or_insert_with(|| compile_shifted(body, dr, dc));
        let Some(ast) = ast else { return false };
        to_boolean(&host.avaliar_em(row, col, ast)).unwrap_or(false)
    }

    // ------------------------------------------------------------ escalas

    fn scale_color(&mut self, host: &dyn RuleHost, rule: &ConditionalRule, cond: &ColorScaleCondition, value: &Value) -> Option<String> {
        let n = numeric(value)?;
        let stats = self.stats_of(host, rule);
        let lo = point_value(&cond.min, stats, 0.0);
        let hi = point_value(&cond.max, stats, 1.0);
        // "Não é maior" inclui o NaN, como o `!(hi > lo)` do TypeScript.
        if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) {
            return Some(cond.max.color.clone());
        }
        let Some(mid_pt) = &cond.mid else {
            return Some(mix_color(&cond.min.color, &cond.max.color, clamp01((n - lo) / (hi - lo))));
        };
        let mid = point_value(mid_pt, stats, 0.5);
        if n <= mid {
            let span = mid - lo;
            return Some(mix_color(&cond.min.color, &mid_pt.color, if span > 0.0 { clamp01((n - lo) / span) } else { 0.0 }));
        }
        let span = hi - mid;
        Some(mix_color(&mid_pt.color, &cond.max.color, if span > 0.0 { clamp01((n - mid) / span) } else { 1.0 }))
    }

    fn bar_for(&mut self, host: &dyn RuleHost, rule: &ConditionalRule, cond: &DataBarCondition, value: &Value) -> Option<BarPaint> {
        let n = numeric(value)?;
        let stats = self.stats_of(host, rule);
        // A barra sempre inclui o zero: com valores negativos ela cresce para
        // os dois lados a partir de um eixo no meio.
        let low = cond.min.as_ref().map_or(stats.min, |p| point_value(p, stats, 0.0)).min(0.0);
        let high = cond.max.as_ref().map_or(stats.max, |p| point_value(p, stats, 1.0)).max(0.0);
        let span = high - low;
        if span.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return None;
        }
        let zero = clamp01((0.0 - low) / span);
        let here = clamp01((n - low) / span);
        let color = if n < 0.0 { cond.negative_color.clone().unwrap_or_else(|| cond.color.clone()) } else { cond.color.clone() };
        Some(BarPaint { from: zero.min(here), to: zero.max(here), color, hide_value: cond.hide_value })
    }

    /// Percorre o que a regra cobre, sem passar do que a aba ocupa: uma regra
    /// pode pegar a coluna inteira, e varrer um milhão de células vazias por
    /// quadro sairia caro.
    fn scan(host: &dyn RuleHost, rule: &ConditionalRule, mut visit: impl FnMut(Value)) {
        let (max_row, max_col) = (host.max_row(), host.max_col());
        for rect in &rule.ranges {
            let (ultima_linha, ultima_coluna) = (rect.r1.min(max_row), rect.c1.min(max_col));
            if rect.r0 > ultima_linha || rect.c0 > ultima_coluna {
                continue;
            }
            for r in rect.r0..=ultima_linha {
                for c in rect.c0..=ultima_coluna {
                    let v = host.valor(r, c);
                    if matches!(&v, Value::Empty | Value::Error(_)) || matches!(&v, Value::Text(t) if t.is_empty()) {
                        continue;
                    }
                    visit(v);
                }
            }
        }
    }

    fn stats_of(&mut self, host: &dyn RuleHost, rule: &ConditionalRule) -> &Stats {
        self.stats.entry(rule.id.clone()).or_insert_with(|| {
            let mut values = Vec::new();
            let mut sum = 0.0;
            Self::scan(host, rule, |v| {
                if let Some(n) = numeric(&v) {
                    values.push(n);
                    sum += n;
                }
            });
            values.sort_by(|a, b| a.total_cmp(b));
            Stats {
                min: values.first().copied().unwrap_or(0.0),
                max: values.last().copied().unwrap_or(0.0),
                avg: if values.is_empty() { 0.0 } else { sum / values.len() as f64 },
                sorted: values,
            }
        })
    }

    /// Contagem por valor, só para repetido/único: monta um texto por célula.
    fn counts_of(&mut self, host: &dyn RuleHost, rule: &ConditionalRule) -> &HashMap<String, usize> {
        self.counts.entry(rule.id.clone()).or_insert_with(|| {
            let mut counts = HashMap::new();
            Self::scan(host, rule, |v| *counts.entry(bucket_key(&v)).or_insert(0) += 1);
            counts
        })
    }
}

// -------------------------------------------------------------- auxiliares

fn fill_gaps(mut base: CondStyle, patch: &CondStyle) -> CondStyle {
    fn preenche<T: Clone>(a: &mut Option<T>, b: &Option<T>) {
        if a.is_none() {
            a.clone_from(b);
        }
    }
    preenche(&mut base.bg, &patch.bg);
    preenche(&mut base.color, &patch.color);
    preenche(&mut base.bold, &patch.bold);
    preenche(&mut base.italic, &patch.italic);
    preenche(&mut base.underline, &patch.underline);
    preenche(&mut base.strike, &patch.strike);
    base
}

/// Texto do campo da regra virando valor: "10" -> 10, "01/01/2026" -> serial.
fn literal(raw: Option<&String>) -> Value {
    parse_literal(raw.map(|s| s.trim()).unwrap_or("")).value
}

fn numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(*n),
        Value::Text(_) => to_number(value).ok(),
        _ => None,
    }
}

/// Chave de agrupamento para repetido/único: texto é comparado sem maiúsculas.
fn bucket_key(value: &Value) -> String {
    match value {
        Value::Text(t) => format!("s:{}", t.to_lowercase()),
        outro => format!("v:{}", to_text(outro)),
    }
}

fn clamp01(n: f64) -> f64 {
    n.clamp(0.0, 1.0)
}

fn point_value(point: &ScalePoint, stats: &Stats, fallback: f64) -> f64 {
    match point.point_type {
        ScalePointType::Min => stats.min,
        ScalePointType::Max => stats.max,
        ScalePointType::Number => point.value.unwrap_or(stats.min),
        ScalePointType::Percent => stats.min + (point.value.unwrap_or(fallback * 100.0) / 100.0) * (stats.max - stats.min),
        ScalePointType::Percentile => percentile(&stats.sorted, point.value.unwrap_or(fallback * 100.0)),
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pos = clamp01(p / 100.0) * (sorted.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    if lo == hi {
        return sorted[lo];
    }
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

/// "#rgb" ou "#rrggbb" -> (r, g, b).
pub fn parse_color(text: &str) -> Option<(u8, u8, u8)> {
    let t = text.trim();
    let hex = t.strip_prefix('#').unwrap_or(t);
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let canal = |s: &str| u8::from_str_radix(s, 16).ok();
    match hex.len() {
        3 => {
            let d: Vec<String> = hex.chars().map(|c| format!("{c}{c}")).collect();
            Some((canal(&d[0])?, canal(&d[1])?, canal(&d[2])?))
        }
        6 => Some((canal(&hex[0..2])?, canal(&hex[2..4])?, canal(&hex[4..6])?)),
        _ => None,
    }
}

/// Mistura duas cores hexadecimais; `t` vai de 0 (a) a 1 (b).
pub fn mix_color(a: &str, b: &str, t: f64) -> String {
    let (Some(ca), Some(cb)) = (parse_color(a), parse_color(b)) else {
        return if t < 0.5 { a.to_string() } else { b.to_string() };
    };
    let k = clamp01(t);
    // `Math.round` do JavaScript: meio arredonda para cima.
    let canal = |x: u8, y: u8| ((x as f64 + (y as f64 - x as f64) * k) + 0.5).floor() as u8;
    format!("#{:02x}{:02x}{:02x}", canal(ca.0, cb.0), canal(ca.1, cb.1), canal(ca.2, cb.2))
}

/// Desloca as referências relativas da fórmula e compila. `None` se não analisar.
fn compile_shifted(body: &str, dr: i32, dc: i32) -> Option<Node> {
    let texto = if dr == 0 && dc == 0 { body.to_string() } else { shift_formula(&format!("={body}"), dr, dc)[1..].to_string() };
    parse_formula(&texto).ok()
}

#[cfg(test)]
mod testes {
    //! Os casos do `test/core/rules.test.ts` (menos os de inserir/excluir linha,
    //! que são do `ops`).
    use super::*;
    use crate::celulas::refs::col_index;
    use crate::planilha::sheet::Sheet;

    impl RuleHost for Sheet {
        fn valor(&self, row: u32, col: u32) -> Value {
            self.get_value(row, col)
        }
        fn avaliar_em(&self, row: u32, col: u32, node: &Node) -> Value {
            Sheet::avaliar_em(self, row, col, node, None)
        }
        fn max_row(&self) -> u32 {
            self.used_rect().r1
        }
        fn max_col(&self) -> u32 {
            self.used_rect().c1
        }
    }

    fn at(r: &str) -> (u32, u32) {
        let i = r.find(|c: char| c.is_ascii_digit()).unwrap();
        (r[i..].parse::<u32>().unwrap() - 1, col_index(&r[..i]).unwrap())
    }

    fn planilha(celulas: &[(&str, &str)]) -> Sheet {
        let mut s = Sheet::new("A");
        for (r, v) in celulas {
            let (row, col) = at(r);
            s.set_cell(row, col, v);
        }
        s
    }

    fn regra(faixa: &str, op: &str, valores: &[&str], estilo: &str) -> ConditionalRule {
        let (a, b) = faixa.split_once(':').unwrap();
        let ((r0, c0), (r1, c1)) = (at(a), at(b));
        let json = format!(
            r#"{{"id":"{faixa}{op}","ranges":[{{"r0":{r0},"c0":{c0},"r1":{r1},"c1":{c1}}}],
               "condition":{{"kind":"cell","op":"{op}","values":{},"style":{estilo}}}}}"#,
            serde_json_lista(valores)
        );
        de_json(&json)
    }

    fn serde_json_lista(v: &[&str]) -> String {
        format!("[{}]", v.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>().join(","))
    }

    fn de_json(texto: &str) -> ConditionalRule {
        serde_json::from_str(texto).expect("regra de teste inválida")
    }

    const AMARELO: &str = r##"{"bg":"#ffff00"}"##;

    fn bg(f: &mut ConditionalFormats, s: &Sheet, r: &str) -> Option<String> {
        let (row, col) = at(r);
        f.format_for(s, row, col, &s.get_value(row, col)).and_then(|x| x.style).and_then(|x| x.bg)
    }

    fn com(regras: Vec<ConditionalRule>) -> ConditionalFormats {
        let mut f = ConditionalFormats::new();
        f.set_rules(regras);
        f
    }

    #[test]
    fn maior_que_pinta_so_quem_passa_do_limite() {
        let s = planilha(&[("A1", "5"), ("A2", "15"), ("A3", "10")]);
        let mut f = com(vec![regra("A1:A3", "greaterThan", &["10"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1"), None);
        assert_eq!(bg(&mut f, &s, "A2").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "A3"), None, "igual ao limite não é maior");
    }

    #[test]
    fn vazia_fica_de_fora_mas_esta_vazia_a_pega() {
        let s = planilha(&[("A2", "3")]);
        let mut f = com(vec![regra("A1:A3", "lessThan", &["10"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1"), None, "vazia não conta como zero");
        assert_eq!(bg(&mut f, &s, "A2").as_deref(), Some("#ffff00"));
        let mut f = com(vec![regra("A1:A3", "isEmpty", &[], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "A2"), None);
    }

    #[test]
    fn entre_texto_contem_e_comeca_com() {
        let s = planilha(&[("A1", "5"), ("A2", "50"), ("B1", "Banana"), ("B2", "Abacaxi")]);
        let mut f = com(vec![regra("A1:A2", "between", &["1", "10"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "A2"), None);
        let mut f = com(vec![regra("B1:B2", "textContains", &["ana"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "B1").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "B2"), None);
        let mut f = com(vec![regra("B1:B2", "textStartsWith", &["aba"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "B1"), None);
        assert_eq!(bg(&mut f, &s, "B2").as_deref(), Some("#ffff00"), "sem maiúsculas");
    }

    #[test]
    fn le_o_valor_calculado_da_formula() {
        let mut s = planilha(&[("A1", "4"), ("A2", "=A1*5")]);
        let mut f = com(vec![regra("A2:A2", "greaterThan", &["10"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A2").as_deref(), Some("#ffff00"));
        s.set_cell(0, 0, "1");
        f.invalidate();
        assert_eq!(bg(&mut f, &s, "A2"), None);
    }

    /// Divergia no porte antigo: o valor da regra era lido com o `parse` do
    /// Rust, e data digitada ou "1,5" nunca comparavam.
    #[test]
    fn data_e_decimal_com_virgula_como_no_typescript() {
        let s = planilha(&[("A1", "10/03/2026"), ("A2", "20/03/2026"), ("B1", "1,2"), ("B2", "1,8")]);
        let mut f = com(vec![regra("A1:A2", "dateBefore", &["15/03/2026"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "A2"), None);
        let mut f = com(vec![regra("B1:B2", "greaterThan", &["1,5"], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "B1"), None);
        assert_eq!(bg(&mut f, &s, "B2").as_deref(), Some("#ffff00"));
    }

    #[test]
    fn repetido_e_unico_olham_o_intervalo_inteiro() {
        let s = planilha(&[("A1", "x"), ("A2", "X"), ("A3", "y")]);
        let mut f = com(vec![regra("A1:A3", "duplicate", &[], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "A2").as_deref(), Some("#ffff00"), "maiúscula não faz outro valor");
        assert_eq!(bg(&mut f, &s, "A3"), None);
        let mut f = com(vec![regra("A1:A3", "unique", &[], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A3").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "A1"), None);
    }

    /// Não existia no porte antigo: a fórmula personalizada nunca pintava.
    #[test]
    fn formula_personalizada_escorrega_junto_com_a_celula() {
        let s = planilha(&[("A1", "um"), ("B1", "sim"), ("A2", "dois"), ("B2", "não"), ("A3", "três"), ("B3", "sim")]);
        let mut f = com(vec![regra("A1:B3", "formula", &["=$B1=\"sim\""], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1").as_deref(), Some("#ffff00"));
        assert_eq!(bg(&mut f, &s, "B1").as_deref(), Some("#ffff00"), "a coluna travada não escorrega");
        assert_eq!(bg(&mut f, &s, "A2"), None);
        assert_eq!(bg(&mut f, &s, "A3").as_deref(), Some("#ffff00"));
    }

    #[test]
    fn formula_que_nao_analisa_nao_pinta() {
        let s = planilha(&[("A1", "1")]);
        let mut f = com(vec![regra("A1:A1", "formula", &["=SOMA(("], AMARELO)]);
        assert_eq!(bg(&mut f, &s, "A1"), None);
    }

    #[test]
    fn cores_alternadas_pintam_linha_sim_linha_nao() {
        let s = planilha(&[("A1", "um"), ("A2", "dois"), ("A3", "três"), ("A4", "quatro")]);
        let mut f = com(vec![regra("A1:A4", "formula", &["=ÉPAR(LIN())"], r##"{"bg":"#f3f3f3"}"##)]);
        assert_eq!(bg(&mut f, &s, "A1"), None);
        assert_eq!(bg(&mut f, &s, "A2").as_deref(), Some("#f3f3f3"));
        assert_eq!(bg(&mut f, &s, "A3"), None);
        assert_eq!(bg(&mut f, &s, "A4").as_deref(), Some("#f3f3f3"));
    }

    #[test]
    fn a_primeira_regra_manda_as_outras_preenchem() {
        let s = planilha(&[("A1", "20")]);
        let mut f = com(vec![
            regra("A1:A1", "greaterThan", &["10"], r##"{"bg":"#ff0000"}"##),
            regra("A1:A1", "greaterThan", &["5"], r##"{"bg":"#00ff00","bold":true}"##),
        ]);
        let estilo = f.format_for(&s, 0, 0, &s.get_value(0, 0)).unwrap().style.unwrap();
        assert_eq!(estilo.bg.as_deref(), Some("#ff0000"));
        assert_eq!(estilo.bold, Some(true), "o negrito da segunda entra porque a primeira não define");
    }

    fn escala() -> ConditionalRule {
        de_json(
            r##"{"id":"e","ranges":[{"r0":0,"c0":0,"r1":2,"c1":0}],
                 "condition":{"kind":"colorScale","min":{"type":"min","color":"#000000"},"max":{"type":"max","color":"#ffffff"}}}"##,
        )
    }

    #[test]
    fn escala_interpola_e_se_refaz() {
        let mut s = planilha(&[("A1", "0"), ("A2", "5"), ("A3", "10")]);
        let mut f = com(vec![escala()]);
        assert_eq!(bg(&mut f, &s, "A1").as_deref(), Some("#000000"));
        assert_eq!(bg(&mut f, &s, "A3").as_deref(), Some("#ffffff"));
        assert_eq!(bg(&mut f, &s, "A2").as_deref(), Some("#808080"));
        s.set_cell(2, 0, "20");
        f.invalidate();
        assert_eq!(bg(&mut f, &s, "A2").as_deref(), Some("#404040"));
    }

    #[test]
    fn barra_cresce_dos_dois_lados_do_zero() {
        let s = planilha(&[("A1", "-10"), ("A2", "0"), ("A3", "10")]);
        let mut f = com(vec![de_json(
            r##"{"id":"b","ranges":[{"r0":0,"c0":0,"r1":2,"c1":0}],
                 "condition":{"kind":"dataBar","color":"#3b78e7","negativeColor":"#d93025"}}"##,
        )]);
        let barra = |f: &mut ConditionalFormats, r: u32| f.format_for(&s, r, 0, &s.get_value(r, 0)).unwrap().bar.unwrap();
        let (neg, zero, pos) = (barra(&mut f, 0), barra(&mut f, 1), barra(&mut f, 2));
        assert_eq!((neg.from, neg.to, neg.color.as_str()), (0.0, 0.5, "#d93025"));
        assert_eq!((zero.from, zero.to), (0.5, 0.5));
        assert_eq!((pos.from, pos.to, pos.color.as_str()), (0.5, 1.0, "#3b78e7"));
    }

    #[test]
    fn mistura_de_cores() {
        assert_eq!(mix_color("#000000", "#ffffff", 0.0), "#000000");
        assert_eq!(mix_color("#000000", "#ffffff", 1.0), "#ffffff");
        assert_eq!(mix_color("#f00", "#00f", 0.5), "#800080");
    }
}
