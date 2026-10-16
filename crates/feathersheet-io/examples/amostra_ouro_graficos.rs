//! Amostra de ouro dos GRÁFICOS: um `.fshz` para avaliar no aplicativo tudo
//! o que o motor de gráficos desenha hoje.
//!
//! - aba "Tipos": um gráfico de cada um dos 25 tipos do motor;
//! - aba "Personalizados": os 10 temas e cada opção do painel "Gráfico >
//!   Personalizar" aplicada sozinha (o título diz o que mudou);
//! - aba "Configurar": um boletim com séries escolhidas, nome na legenda e
//!   conta por categoria (Contar, Média), como na aba Configurar do painel.
//!
//! Uso (a partir de `feathersheet/crates/feathersheet-io`):
//!   systemd-run --user --scope -p MemoryMax=8G cargo run --release \
//!     --example amostra_ouro_graficos -- [pasta-de-saida]

use std::path::PathBuf;

use feather_charts::estilo::*;
use feathersheet_core::celulas::locale::{set_locale_config, LocaleConfig};
use feathersheet_core::celulas::refs::parse_cell_ref;
use feathersheet_core::grafico::chart::{ChartRange, ChartType};
use feathersheet_core::planilha::workbook::Workbook;
use serde_json::{json, Value};

const LARGURA: f64 = 480.0;
const ALTURA: f64 = 300.0;

fn put(wb: &mut Workbook, aba: usize, a1: &str, texto: &str) {
    let (r, c) = parse_cell_ref(a1).expect("célula");
    wb.escrever_celula(aba, r, c, texto.to_string());
}
fn put_rc(wb: &mut Workbook, aba: usize, r: u32, c: u32, texto: &str) {
    wb.escrever_celula(aba, r, c, texto.to_string());
}
fn rng(r0: u32, c0: u32, r1: u32, c1: u32) -> ChartRange {
    ChartRange { r0, c0, r1, c1 }
}
fn cor(v: u32) -> Option<CorEstilo> {
    Some(CorEstilo::hex(v))
}

/// Um gráfico gravado no formato do `.fshz` (o mesmo JSON do painel).
struct G {
    tipo: ChartType,
    titulo: String,
    range: ChartRange,
    cat: bool,
    estilo: Option<EstiloGrafico>,
    extra: Value,
}
fn g(tipo: ChartType, titulo: &str, range: ChartRange) -> G {
    G { tipo, titulo: titulo.into(), range, cat: true, estilo: None, extra: json!({}) }
}
fn com(tipo: ChartType, titulo: &str, range: ChartRange, f: impl FnOnce(&mut EstiloGrafico)) -> G {
    let mut e = EstiloGrafico::default();
    f(&mut e);
    G { tipo, titulo: titulo.into(), range, cat: true, estilo: Some(e), extra: json!({}) }
}

/// Grava a lista na aba, em grade de 3 colunas abaixo da linha `y0` (px).
fn gravar(wb: &mut Workbook, aba: usize, lista: Vec<G>, y0: f64) -> usize {
    let n = lista.len();
    let json: Vec<Value> = lista.into_iter().enumerate().map(|(i, g)| {
        let x = 20.0 + (i % 3) as f64 * (LARGURA + 20.0);
        let y = y0 + (i / 3) as f64 * (ALTURA + 30.0);
        let mut nativo = json!({});
        if let Some(e) = &g.estilo {
            nativo["estilo"] = serde_json::to_value(e).unwrap();
        }
        let mut v = json!({
            "id": format!("ouro-{aba}-{i:02}"),
            "type": serde_json::to_value(g.tipo).unwrap(),
            "title": g.titulo,
            "range": {"r0": g.range.r0, "c0": g.range.c0, "r1": g.range.r1, "c1": g.range.c1},
            "seriesInColumns": true,
            "hasHeaders": true,
            "hasCategoryLabels": g.cat,
            "aggregate": false,
            "aggregateFn": "sum",
            "style": {"showTitle": true, "legendPosition": "bottom"},
            "position": {"x": x, "y": y, "width": LARGURA, "height": ALTURA},
            "_featherNative": nativo,
        });
        if let (Some(o), Some(extra)) = (v.as_object_mut(), g.extra.as_object()) {
            for (k, val) in extra {
                if k == "_featherNative" {
                    for (k2, v2) in val.as_object().into_iter().flatten() {
                        o["_featherNative"][k2] = v2.clone();
                    }
                } else if k == "style" {
                    for (k2, v2) in val.as_object().into_iter().flatten() {
                        o["style"][k2] = v2.clone();
                    }
                } else {
                    o.insert(k.clone(), val.clone());
                }
            }
        }
        v
    }).collect();
    wb.editar(aba, |s| s.g = Some(serde_json::to_string(&json).unwrap()));
    n
}

/// Vendas/Meta/Custo por mês em A3:D9 (as personalizações usam esta tabela).
fn tabela_vendas(wb: &mut Workbook, aba: usize) -> ChartRange {
    let t = [("Mês", "Vendas", "Meta", "Custo"), ("Jan", "120", "100", "80"), ("Fev", "135", "110", "85"),
        ("Mar", "150", "120", "90"), ("Abr", "110", "130", "95"), ("Mai", "170", "140", "100"), ("Jun", "190", "150", "105")];
    for (i, l) in t.iter().enumerate() {
        for (j, v) in [l.0, l.1, l.2, l.3].iter().enumerate() {
            put_rc(wb, aba, 2 + i as u32, j as u32, v);
        }
    }
    rng(2, 0, 8, 3)
}

fn aba_tipos(wb: &mut Workbook, aba: usize) -> usize {
    put(wb, aba, "A1", "Um gráfico de cada tipo do motor (25). Dados nas tabelas da linha 3; os gráficos ficam abaixo.");
    let t1 = tabela_vendas(wb, aba);
    put(wb, aba, "F3", "X");
    put(wb, aba, "G3", "Y");
    for (i, (x, y)) in [(1, 2), (2, 4), (3, 5), (4, 4), (5, 7), (6, 9), (7, 8)].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 5, &x.to_string());
        put_rc(wb, aba, 3 + i as u32, 6, &y.to_string());
    }
    for (j, h) in ["X", "Y", "Tamanho"].iter().enumerate() {
        put_rc(wb, aba, 2, 8 + j as u32, h);
    }
    for (i, (x, y, z)) in [(1, 5, 10), (2, 8, 20), (3, 4, 15), (4, 9, 30), (5, 6, 25), (6, 7, 12)].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 8, &x.to_string());
        put_rc(wb, aba, 3 + i as u32, 9, &y.to_string());
        put_rc(wb, aba, 3 + i as u32, 10, &z.to_string());
    }
    put(wb, aba, "M3", "Amostra");
    for (i, v) in [2, 3, 3, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 7, 7, 8, 9, 10, 12, 15].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 12, &v.to_string());
    }
    put(wb, aba, "O3", "Etapa");
    put(wb, aba, "P3", "Variação");
    for (i, (n, v)) in [("Início", 100), ("Receita", 60), ("Custos", -40), ("Impostos", -15), ("Bônus", 25), ("Fim", 130)].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 14, n);
        put_rc(wb, aba, 3 + i as u32, 15, &v.to_string());
    }
    for (j, h) in ["Dia", "Abertura", "Máxima", "Mínima", "Fechamento"].iter().enumerate() {
        put_rc(wb, aba, 2, 17 + j as u32, h);
    }
    for (i, (d, a, mx, mn, f)) in [("D1", 10, 14, 9, 13), ("D2", 13, 15, 11, 12), ("D3", 12, 16, 12, 15), ("D4", 15, 18, 14, 17), ("D5", 17, 19, 13, 14), ("D6", 14, 17, 13, 16)].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 17, d);
        for (j, v) in [a, mx, mn, f].iter().enumerate() {
            put_rc(wb, aba, 3 + i as u32, 18 + j as u32, &v.to_string());
        }
    }
    for (j, h) in ["Tarefa", "Início", "Término"].iter().enumerate() {
        put_rc(wb, aba, 2, 23 + j as u32, h);
    }
    for (i, (n, s, d)) in [("Projeto", 0, 4), ("Obra", 3, 9), ("Testes", 8, 11), ("Entrega", 11, 12)].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 23, n);
        put_rc(wb, aba, 3 + i as u32, 24, &s.to_string());
        put_rc(wb, aba, 3 + i as u32, 25, &d.to_string());
    }
    for (j, h) in ["Região", "Seg", "Ter", "Qua", "Qui"].iter().enumerate() {
        put_rc(wb, aba, 2, 27 + j as u32, h);
    }
    for (i, n) in ["Norte", "Sul", "Leste", "Oeste", "Centro"].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 27, n);
        for j in 0..4u32 {
            put_rc(wb, aba, 3 + i as u32, 28 + j, &(((i as u32 * 7 + j * 13) % 17) + 1).to_string());
        }
    }
    for (j, h) in ["Grupo", "X", "Y", "Z"].iter().enumerate() {
        put_rc(wb, aba, 2, 33 + j as u32, h);
    }
    for i in 0..8u32 {
        put_rc(wb, aba, 3 + i, 33, if i < 4 { "G1" } else { "G2" });
        put_rc(wb, aba, 3 + i, 34, &(i % 4 + 1).to_string());
        put_rc(wb, aba, 3 + i, 35, &((i * 3) % 7 + 1).to_string());
        put_rc(wb, aba, 3 + i, 36, &((i * 5) % 6 + 2).to_string());
    }
    use ChartType as T;
    let mut lista = vec![
        g(T::Column, "Colunas", t1),
        g(T::ColumnStacked, "Colunas empilhadas", t1),
        g(T::ColumnStacked100, "Colunas 100%", t1),
        g(T::Bar, "Barras", t1),
        g(T::BarStacked, "Barras empilhadas", t1),
        g(T::BarStacked100, "Barras 100%", t1),
        g(T::Line, "Linhas", t1),
        g(T::StepLine, "Linhas em degraus", t1),
        g(T::StepLineStacked, "Degraus empilhados", t1),
        g(T::StepLineStacked100, "Degraus 100%", t1),
        g(T::Area, "Área", t1),
        g(T::StepArea, "Área em degraus", t1),
        g(T::StepAreaStacked, "Área em degraus empilhada", t1),
        g(T::StepAreaStacked100, "Área em degraus 100%", t1),
        g(T::Pie, "Pizza", rng(2, 0, 8, 1)),
        g(T::Radar, "Radar", t1),
        g(T::Combo, "Combinado", t1),
        g(T::Scatter, "Dispersão", rng(2, 5, 9, 6)),
        g(T::Bubble, "Bolhas", rng(2, 8, 8, 10)),
        g(T::Histogram, "Histograma", rng(2, 12, 22, 12)),
        g(T::Waterfall, "Cascata", rng(2, 14, 8, 15)),
        g(T::Candlestick, "Velas (candlestick)", rng(2, 17, 8, 21)),
        g(T::Gantt, "Gantt", rng(2, 23, 6, 25)),
        g(T::Heatmap, "Mapa de calor", rng(2, 27, 7, 31)),
        g(T::Line3d, "Linha 3D", rng(2, 34, 10, 36)),
    ];
    for x in &mut lista {
        if matches!(x.tipo, T::Scatter | T::Histogram | T::Line3d) {
            x.cat = false;
        }
    }
    gravar(wb, aba, lista, 560.0)
}

fn aba_personalizados(wb: &mut Workbook, aba: usize) -> usize {
    put(wb, aba, "A1", "Cada gráfico mostra UMA opção do painel Gráfico > Personalizar (o título diz qual). Todos usam a tabela A3:D9.");
    let t = tabela_vendas(wb, aba);
    use ChartType as T;
    let mut lista: Vec<G> = temas().iter()
        .map(|tema| com(T::Column, &format!("Tema {}", tema.nome), t, |e| e.tema = Some(tema.id.into())))
        .collect();
    lista.extend([
        com(T::Column, "Cor por ponto (Abr em vermelho) e rótulos", t, |e| {
            e.serie_mut(0).pontos.entry(3).or_default().cor = cor(0xd81b60);
            e.rotulos.visivel = Some(true);
        }),
        com(T::Column, "Rótulos no centro, em moeda", t, |e| {
            e.rotulos.visivel = Some(true);
            e.rotulos.posicao = Some(PosicaoRotulo::Centro);
            e.rotulos.formato = Some(FormatoNum::Moeda { casas: 0 });
        }),
        com(T::Column, "Eixo Y: mín 50, máx 200, passo 25, moeda, títulos", t, |e| {
            e.eixo_y.minimo = Some(50.0);
            e.eixo_y.maximo = Some(200.0);
            e.eixo_y.unidade_principal = Some(25.0);
            e.eixo_y.formato = Some(FormatoNum::Moeda { casas: 0 });
            e.eixo_y.titulo = Some("Valor".into());
            e.eixo_x.titulo = Some("Mês".into());
        }),
        com(T::Line, "Eixo Y logarítmico", t, |e| e.eixo_y.logaritmica = true),
        com(T::Column, "Eixo Y invertido", t, |e| e.eixo_y.invertido = true),
        com(T::Line, "Grades principal e secundária tracejadas", t, |e| {
            e.eixo_y.grade_principal = Linha { visivel: Some(true), cor: cor(0x2b7a78), espessura: Some(1.0), traco: Some(Traco::Tracejado) };
            e.eixo_y.grade_secundaria = Linha { visivel: Some(true), cor: cor(0xbfbfbf), espessura: Some(0.5), traco: Some(Traco::Pontilhado) };
            e.eixo_x.grade_principal = Linha { visivel: Some(true), cor: cor(0xbfbfbf), espessura: Some(0.5), traco: Some(Traco::Continuo) };
        }),
        com(T::Line, "Linhas: marcadores, espessura, suavizar, traço", t, |e| {
            let s = e.serie_mut(0);
            s.marcador = Some(FormaMarcador::Circulo);
            s.tamanho_marcador = Some(8.0);
            s.espessura_linha = Some(3.5);
            s.suavizar = Some(true);
            let s = e.serie_mut(1);
            s.traco = Some(Traco::Tracejado);
            s.marcador = Some(FormaMarcador::Losango);
            e.serie_mut(2).marcador = Some(FormaMarcador::Triangulo);
        }),
        com(T::Line, "Custo no eixo secundário (direita)", t, |e| {
            e.serie_mut(2).eixo_secundario = true;
            e.eixo_y2.titulo = Some("Custo".into());
        }),
        com(T::Area, "Área com transparência", t, |e| {
            for i in 0..3 {
                e.serie_mut(i).opacidade = Some(0.45);
            }
        }),
        com(T::Column, "Barras: espaçamento 20%, separadas, cantos 6", t, |e| {
            e.barras.espacamento = Some(20.0);
            e.barras.sobreposicao = Some(-20.0);
            e.barras.raio = Some(6.0);
        }),
        com(T::Column, "Barras sobrepostas 50%", t, |e| e.barras.sobreposicao = Some(50.0)),
        com(T::Column, "Fundo degradê, sombra, borda, cantos, margens", t, |e| {
            e.area_grafico.preenchimento = Some(Preenchimento::Degrade { inicio: CorEstilo::hex(0xe3f2fd), fim: CorEstilo::hex(0xffffff), angulo: 90.0 });
            e.area_grafico.borda = Linha { visivel: Some(true), cor: cor(0x0b6e99), espessura: Some(2.0), traco: None };
            e.area_grafico.sombra = Some(Sombra { cor: CorEstilo::hex(0x7f7f7f), desfoque: 6.0, dx: 3.0, dy: 3.0 });
            e.area_grafico.raio = Some(14.0);
            e.margens_grafico = Margens { superior: Some(18.0), inferior: Some(18.0), esquerda: Some(18.0), direita: Some(18.0) };
        }),
        com(T::Column, "Área de plotagem colorida com borda", t, |e| {
            e.area_plotagem.preenchimento = Some(Preenchimento::Solido { cor: CorEstilo::hex(0xfff8e1) });
            e.area_plotagem.borda = Linha { visivel: Some(true), cor: cor(0xe8710a), espessura: Some(1.0), traco: Some(Traco::Tracejado) };
        }),
        com(T::Column, "Título à esquerda, subtítulo, fonte com serifa", t, |e| {
            e.titulo.alinhamento = Some(Alinhamento::Esquerda);
            e.titulo.fonte.negrito = Some(true);
            e.titulo.fonte.tamanho = Some(20.0);
            e.subtitulo.texto = Some("1º semestre de 2026".into());
            e.fonte.familia = Some("Noto Serif, DejaVu Serif, Liberation Serif, Georgia, serif".into());
        }),
        com(T::Column, "Legenda à direita", t, |e| e.legenda.posicao = Some(PosicaoLegenda::Direita)),
        com(T::Column, "Legenda em cima", t, |e| e.legenda.posicao = Some(PosicaoLegenda::Superior)),
        com(T::Column, "Sem legenda", t, |e| e.legenda.posicao = Some(PosicaoLegenda::Nenhuma)),
        com(T::Column, "Legenda: itens renomeados, um oculto, ordem trocada", t, |e| {
            e.legenda.itens.entry(0).or_default().texto = Some("Vendido".into());
            e.legenda.itens.entry(1).or_default().oculto = true;
            e.legenda.ordem = vec![2, 0, 1];
        }),
        com(T::Pie, "Pizza: fatia destacada, rótulos com %", rng(2, 0, 8, 1), |e| {
            e.serie_mut(0).pontos.entry(5).or_default().destacar = Some(18.0);
            e.rotulos.visivel = Some(true);
            e.rotulos.percentual = Some(true);
            e.rotulos.categoria = Some(true);
        }),
        com(T::ColumnStacked, "Colunas empilhadas com rótulos", t, |e| e.rotulos.visivel = Some(true)),
        com(T::ColumnStacked100, "Colunas 100% com tema Oceano", t, |e| e.tema = Some("oceano".into())),
        com(T::BarStacked, "Barras empilhadas", t, |_| {}),
        com(T::BarStacked100, "Barras 100% com rótulos", t, |e| e.rotulos.visivel = Some(true)),
        com(T::Bar, "Barras com rótulos fora", t, |e| {
            e.rotulos.visivel = Some(true);
            e.rotulos.posicao = Some(PosicaoRotulo::ForaFim);
        }),
        com(T::Column, "Rótulos do eixo X girados 45°", t, |e| e.eixo_x.rotacao = Some(-45.0)),
        com(T::Column, "Fonte maior e colorida", t, |e| {
            e.fonte.tamanho = Some(15.0);
            e.fonte.cor = cor(0x1f3a5f);
        }),
        com(T::Column, "Cores das séries trocadas e contorno", t, |e| {
            for (i, c) in [0x188038u32, 0xfbbc04, 0x7b61ff].iter().enumerate() {
                let s = e.serie_mut(i);
                s.cor = cor(*c);
                s.contorno = cor(0x202020);
                s.espessura_contorno = Some(1.5);
            }
        }),
    ]);
    // Dispersão com marcadores (X/Y em F3:G10).
    put(wb, aba, "F3", "X");
    put(wb, aba, "G3", "Y");
    for (i, (x, y)) in [(1, 2), (2, 4), (3, 5), (4, 4), (5, 7), (6, 9), (7, 8)].iter().enumerate() {
        put_rc(wb, aba, 3 + i as u32, 5, &x.to_string());
        put_rc(wb, aba, 3 + i as u32, 6, &y.to_string());
    }
    let mut disp = com(T::Scatter, "Dispersão: losangos tamanho 10", rng(2, 5, 9, 6), |e| {
        let s = e.serie_mut(0);
        s.marcador = Some(FormaMarcador::Losango);
        s.tamanho_marcador = Some(10.0);
    });
    disp.cat = false;
    lista.push(disp);
    gravar(wb, aba, lista, 260.0)
}

fn aba_configurar(wb: &mut Workbook, aba: usize) -> usize {
    put(wb, aba, "A1", "Aba Configurar do painel: séries escolhidas, nome na legenda e conta por categoria.");
    let alunos = [
        ("Ana", "8", "9", "7", "Aprovado"), ("Bruno", "5", "6", "4", "Reprovado"), ("Carla", "9", "8", "10", "Aprovado"),
        ("Daniel", "4", "5", "6", "Reprovado"), ("Eduarda", "7", "7", "8", "Aprovado"), ("Felipe", "6", "4", "5", "Reprovado"),
        ("Gabriela", "10", "9", "9", "Aprovado"), ("Heitor", "3", "5", "4", "Reprovado"),
    ];
    for (j, h) in ["Aluno", "Matrícula", "Prova 1", "Prova 2", "Trabalho", "Situação"].iter().enumerate() {
        put_rc(wb, aba, 2, j as u32, h);
    }
    for (i, (n, p1, p2, t, s)) in alunos.iter().enumerate() {
        let r = 3 + i as u32;
        put_rc(wb, aba, r, 0, n);
        put_rc(wb, aba, r, 1, &format!("{}", 2026001 + i));
        put_rc(wb, aba, r, 2, p1);
        put_rc(wb, aba, r, 3, p2);
        put_rc(wb, aba, r, 4, t);
        put_rc(wb, aba, r, 5, s);
    }
    let t = rng(2, 0, 10, 5);
    use ChartType as T;
    let mut empilhado = g(T::ColumnStacked, "Prova 1, Prova 2 e Trabalho por aluno (Matrícula e Situação ocultas)", t);
    empilhado.extra = json!({
        "categoryColumnIndex": 0,
        "style": {"hiddenSeries": [0, 4]},
        "_featherNative": {"nomes_origem": {"3": "Trabalho em grupo"}},
    });
    let mut pizza = g(T::Pie, "Alunos por situação (categorias = Situação, Contar)", t);
    pizza.extra = json!({
        "categoryColumnIndex": 5,
        "aggregate": true,
        "aggregateBySeries": {"1": "count"},
        "style": {"hiddenSeries": [0, 2, 3, 4]},
        "_featherPieSeries": "1 — Contagem de Prova 1",
    });
    let mut media = g(T::Column, "Média das notas por situação (Média)", t);
    media.extra = json!({
        "categoryColumnIndex": 5,
        "aggregate": true,
        "aggregateBySeries": {"1": "avg", "2": "avg", "3": "avg"},
        "style": {"hiddenSeries": [0, 4]},
    });
    let mut desvio = g(T::Column, "Desvio padrão por situação", t);
    desvio.extra = json!({
        "categoryColumnIndex": 5,
        "aggregate": true,
        "aggregateBySeries": {"1": "stddev", "2": "stddev", "3": "stddev"},
        "style": {"hiddenSeries": [0, 4]},
    });
    gravar(wb, aba, vec![empilhado, pizza, media, desvio], 260.0)
}

fn main() -> Result<(), String> {
    set_locale_config(LocaleConfig::pt_br());
    let saida: PathBuf = std::env::args().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Documentos/FeatherSheet"));
    std::fs::create_dir_all(&saida).map_err(|e| e.to_string())?;
    let mut wb = Workbook::new();
    wb.renomear_aba(0, "Tipos");
    let i_pers = wb.criar_aba("Personalizados");
    let i_conf = wb.criar_aba("Configurar");
    let n1 = aba_tipos(&mut wb, 0);
    let n2 = aba_personalizados(&mut wb, i_pers);
    let n3 = aba_configurar(&mut wb, i_conf);
    wb.calcular_em_paralelo();
    let arquivo = saida.join("amostra-ouro-graficos.fshz");
    feathersheet_io::da_workbook::pasta_gravar_arquivo(&wb, 0, &arquivo)?;
    println!("{} — Tipos: {n1}, Personalizados: {n2}, Configurar: {n3} gráficos.", arquivo.display());
    Ok(())
}
