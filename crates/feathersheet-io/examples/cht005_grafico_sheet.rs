//! Exemplo real: células -> extrator FeatherSheet -> contrato -> SVG Drawing.
//! stdout: somente SVG; diagnósticos vão para stderr.
use std::collections::HashMap;

use feathersheet_core::grafico::chart::{AggregateFn, ChartConfig, ChartRange, ChartType};
use feathersheet_core::Value;
use feathersheet_io::grafico_compartilhado::extrair_grafico_com;

fn main() -> Result<(), String> {
    let mut celulas = HashMap::new();
    for (c, texto) in ["Mês", "Vendas", "Meta"].iter().enumerate() {
        celulas.insert((0, c as u32), Value::Text((*texto).to_owned()));
    }
    for (r, (mes, vendas, meta)) in [
        ("Janeiro", 80., 120.),
        ("Fevereiro", 110., 130.),
        ("Janeiro", 40., 120.),
        ("Março", 150., 140.),
    ].iter().enumerate() {
        celulas.insert((r as u32 + 1, 0), Value::Text((*mes).to_owned()));
        celulas.insert((r as u32 + 1, 1), Value::Number(*vendas));
        celulas.insert((r as u32 + 1, 2), Value::Number(*meta));
    }
    let config = ChartConfig {
        tipo: ChartType::Column,
        title: "Vendas mensais".into(),
        range: ChartRange { r0: 0, c0: 0, r1: 4, c1: 2 },
        series_in_columns: true,
        has_headers: true,
        has_category_labels: true,
        aggregate: true,
        aggregate_fn: Some(AggregateFn::Sum),
        ..Default::default()
    };
    let contrato = extrair_grafico_com(
        &|r, c| celulas.get(&(r, c)).cloned().unwrap_or(Value::Empty),
        &config,
    ).map_err(|e| format!("{e:?}"))?;
    assert_eq!(contrato.grafico.categorias, ["Janeiro", "Fevereiro", "Março"]);
    assert_eq!(contrato.grafico.series[0].valores, [120., 110., 150.]);
    assert_eq!(contrato.grafico.series[1].valores, [240., 130., 140.]);
    assert_eq!(contrato.metadados_series.len(), 2);
    let cena = contrato.cena(800., 450.).map_err(|e| format!("{e:?}"))?;
    let svg = feather_draw::exportar_svg(&cena).map_err(|e| format!("{e:?}"))?;
    eprintln!("CHT-005: {} categorias, {} séries, {} figuras",
        contrato.grafico.categorias.len(), contrato.metadados_series.len(), cena.figuras.len());
    print!("{svg}");
    Ok(())
}
