//! Construção e configuração avançada da tabela dinâmica, inteiramente no Rust.
use super::*;
use std::collections::HashSet;
use feathersheet_core::dinamica::{
    campo_calculado::{self, Expr},
    dinamica_layout::{self, InfoLinha, Layout, Opcoes, Resultado, TipoLinha},
};
use serde_json::{json, Value as Json};

#[derive(Debug, Clone)]
pub(super) struct Calculado {
    pub nome: String,
    pub formula: String,
}

/// Um filtro é uma lista de valores explicitamente EXCLUÍDOS. Evita ocultar
/// automaticamente valores novos ou não apresentados no popover.
pub(super) fn opcoes(e: &Estado) -> Opcoes {
    Opcoes {
        layout: e.layout,
        repetir_rotulos: e.repetir_rotulos,
        subtotais: e.subtotais,
        recolhidos: e.recolhidos.iter().cloned().collect(),
        filtros: e.filtros.iter().filter(|f| !f.valores.is_empty()).cloned().collect(),
    }
}

pub(super) fn opcoes_de_json(meta: &Json) -> (Layout, bool, bool, Vec<String>, Vec<(i64, Vec<String>, bool)>) {
    let layout = if meta["layout"] == "compacto" { Layout::Compacto } else { Layout::Tabular };
    let repetir = meta["repetirRotulos"].as_bool().unwrap_or(true);
    let subtotais = meta["subtotais"].as_bool().unwrap_or(false);
    let recolhidos = meta["recolhidos"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    let filtros = meta["filtros"].as_array().map(|a| a.iter().filter_map(|v| {
        let col = v["col"].as_i64()?;
        let valores: Vec<String> = v["valores"].as_array()?.iter()
            .filter_map(|v| v.as_str().map(str::to_owned)).collect();
        Some((col, valores, v["excluir"].as_bool().unwrap_or(false)))
    }).collect()).unwrap_or_default();
    (layout, repetir, subtotais, recolhidos, filtros)
}

pub(super) fn configuracao_avancada(e: &Estado) -> Result<DinamicaConfig, String> {
    if e.linhas.is_empty() && e.colunas.is_empty() {
        return Err("Escolha ao menos um campo em Linhas ou Colunas.".into());
    }
    if e.valores.is_empty() {
        return Err("Escolha ao menos uma medida em Valores.".into());
    }
    let max = (e.origem.c1 as usize).checked_add(e.calculados.len())
        .ok_or("Quantidade de colunas fora dos limites.")?;
    if max >= MAX_COLS as usize {
        return Err("Não existe espaço para os campos calculados além da última coluna.".into());
    }
    if e.linhas.iter().chain(e.colunas.iter()).chain(e.valores.iter().map(|m| &m.coluna))
        .any(|&c| c < e.origem.c0 as usize || c > max) {
        return Err("Um campo da dinâmica está fora do intervalo de origem.".into());
    }
    Ok(DinamicaConfig {
        origem: Rect::new(e.origem.r0, e.origem.c0, e.origem.r1, max as u32),
        linhas: e.linhas.clone(),
        colunas: e.colunas.clone(),
        valores: e.valores.clone(),
        agrupar_data: e.agrupar_data,
    })
}

/// Nenhuma leitura faz uma cópia densa do intervalo; somente as EXPRESSÕES
/// dos campos virtuais são compiladas uma vez, antes de varrer as linhas.
pub(super) fn calcular(app: &App, e: &Estado) -> Result<Resultado, String> {
    let config = configuracao_avancada(e)?;
    let nomes_reais = &e.nomes[..e.nomes.len().saturating_sub(e.calculados.len())];
    let exprs: Vec<Expr> = e.calculados.iter()
        .map(|c| campo_calculado::compilar(&c.formula, nomes_reais))
        .collect::<Result<_, _>>()?;
    let primeira_virtual = e.origem.c1 as usize + 1;
    let r0 = e.origem.r0;
    let c0 = e.origem.c0;
    let leitor = |r: usize, c: usize| -> Value {
        let absoluta = c0 as usize + c;
        if absoluta < primeira_virtual {
            return app.workbook.valor(e.folha_origem, r0 + r as u32, c0 + c as u32);
        }
        let i = absoluta - primeira_virtual;
        if r == 0 { return Value::Text(e.calculados[i].nome.clone()); }
        let numero = campo_calculado::avaliar(&exprs[i], &|campo| {
            let v = app.workbook.valor(e.folha_origem, r0 + r as u32, c0 + campo as u32);
            campo_calculado::numero_de(&v)
        });
        Value::Number(numero)
    };
    Ok(dinamica_layout::montar_com_extras(
        (e.origem.r1 - e.origem.r0 + 1) as usize,
        leitor,
        &config,
        &opcoes(e),
        &extras_vigentes(e),
    ))
}

/// Os extras lidos do arquivo valem para os campos com que vieram: mudou
/// um campo de lugar, as opções por campo/medida caem (sobram os totais
/// gerais), para não aplicar "% do total" à medida errada.
pub(super) fn extras_vigentes(e: &Estado) -> Extras {
    let medidas: Vec<usize> = e.valores.iter().map(|m| m.coluna).collect();
    let (linhas, colunas, valores) = &e.extras_campos;
    if *linhas == e.linhas && *colunas == e.colunas && *valores == medidas {
        return e.extras.clone();
    }
    Extras {
        totais_gerais_linhas: e.extras.totais_gerais_linhas,
        totais_gerais_colunas: e.extras.totais_gerais_colunas,
        ..Extras::default()
    }
}

pub(super) fn mapa_json(linhas: &[InfoLinha], rotulos: usize) -> Json {
    json!({
        "rotulos": rotulos,
        "linhas": linhas.iter().map(|linha| {
            let tipo = match linha.tipo {
                TipoLinha::Cabecalho => "c",
                TipoLinha::Grupo => "g",
                TipoLinha::Item => "i",
                TipoLinha::Subtotal => "s",
                TipoLinha::Total => "t",
            };
            json!([tipo, linha.nivel, linha.marcas.iter()
                .map(|m| json!([m.coluna, m.caminho, m.recolhido]))
                .collect::<Vec<_>>()])
        }).collect::<Vec<_>>(),
    })
}

/// Colunas virtuais são negativas no contrato TypeScript (a primeira = -1).
/// Internamente, o Rust trabalha com índices absolutos positivos.
fn coluna_persistida(e: &Estado, coluna: usize) -> i64 {
    if coluna > e.origem.c1 as usize {
        -((coluna - e.origem.c1 as usize) as i64)
    } else {
        coluna as i64
    }
}

pub(super) fn gravar_campos(meta: &mut Json, e: &Estado) {
    meta["linhas"] = json!(e.linhas.iter().map(|&col| coluna_persistida(e,col)).collect::<Vec<_>>());
    meta["colunas"] = json!(e.colunas.iter().map(|&col| coluna_persistida(e,col)).collect::<Vec<_>>());
    meta["valores"] = json!(e.valores.iter().map(|m| {
        let como = match m.op {
            AgregacaoOp::Soma => "soma", AgregacaoOp::Contagem => "contagem",
            AgregacaoOp::Media => "media", AgregacaoOp::Min => "minimo",
            AgregacaoOp::Max => "maximo", AgregacaoOp::DesvioPadrao => "desviopadrao",
            AgregacaoOp::Moda => "moda",
        };
        json!({"col": coluna_persistida(e,m.coluna), "como": como})
    }).collect::<Vec<_>>());
    let parte = match e.agrupar_data {
        Some(AgruparData::Mes) => Some("mes"),
        Some(AgruparData::Trimestre) => Some("trimestre"),
        Some(AgruparData::Ano) => Some("ano"),
        None => None,
    };
    meta["extras"] = serde_json::to_value(extras_vigentes(e)).unwrap_or_default();
    meta["datas"] = json!(parte.map(|nome| {
        e.linhas.iter().chain(e.colunas.iter())
            .map(|&col| json!({"col":coluna_persistida(e,col),"parte":nome}))
            .collect::<Vec<_>>()
    }).unwrap_or_default());
}

pub(super) fn gravar_opcoes(meta: &mut Json, e: &Estado, mapa: Json) {
    meta["layout"] = json!(if e.layout == Layout::Compacto { "compacto" } else { "tabular" });
    meta["repetirRotulos"] = json!(e.repetir_rotulos);
    meta["subtotais"] = json!(e.subtotais);
    meta["tema"] = json!(e.tema);
    meta["recolhidos"] = json!(e.recolhidos);
    meta["filtros"] = json!(e.filtros.iter().map(|f| {
        let mut valores: Vec<_> = f.valores.iter().cloned().collect();
        valores.sort();
        json!({"col": coluna_persistida(e,f.coluna), "valores": valores, "excluir":f.excluir})
    }).collect::<Vec<_>>());
    meta["calculados"] = json!(e.calculados.iter().map(|c| {
        json!({"nome":c.nome, "formula":c.formula})
    }).collect::<Vec<_>>());
    meta["mapa"] = mapa;
}

/// Ordenação estável, eliminando duplicatas, com um teto de EXIBIÇÃO:
/// não filtra valores que ficaram de fora da amostra.
pub(super) fn opcoes_de_filtro(app: &App, e: &Estado, coluna: usize) -> (Vec<(String, String)>,bool) {
    // Chave = o texto que o filtro compara; rótulo = como a célula aparece
    // (data como data, não 46296).
    let mut encontrados = std::collections::BTreeMap::new();
    let aba = app.workbook.aba(e.folha_origem);
    // Só uma amostra até 20 mil linhas e 160 valores distintos para não
    // bloquear a UI. A lista cortada nunca permite filtro inclusivo:
    // os valores não amostrados desapareceriam do resultado silenciosamente.
    let ultima = e.origem.r1.min(e.origem.r0.saturating_add(20_000));
    let mut cortado = ultima < e.origem.r1;
    for r in (e.origem.r0 + 1)..=ultima {
        if encontrados.len() >= 160 {
            cortado = true;
            break;
        }
        let valor = app.workbook.valor(e.folha_origem, r, coluna as u32);
        let chave = if valor == Value::Empty { "(Vazio)".to_string() } else {
            let txt = to_text(&valor);
            if txt.trim().is_empty() { "(Vazio)".to_string() } else { txt }
        };
        if !encontrados.contains_key(&chave) {
            let estilo = aba.and_then(|a| a.sheet.estilo_resolvido(r, coluna as u32));
            let rotulo = if chave == "(Vazio)" { chave.clone() } else {
                feathersheet_core::planilha::bloco::texto_formatado(&valor, estilo.as_ref())
            };
            encontrados.insert(chave, rotulo);
        }
    }
    (encontrados.into_iter().collect(), cortado)
}

pub(super) fn conferir_nomes_calculados(e: &Estado) -> Result<(), String> {
    let mut vistos = HashSet::new();
    for c in &e.calculados {
        if c.nome.trim().is_empty() || c.formula.trim().is_empty() {
            return Err("Informe o nome e a fórmula de cada campo calculado.".into());
        }
        if !vistos.insert(c.nome.trim().to_lowercase()) {
            return Err("Existem nomes duplicados de campos calculados.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn mapa_preserva_marcas_tipo_e_nivel() {
        let info = InfoLinha {
            tipo: TipoLinha::Grupo, nivel: 2,
            marcas: vec![dinamica_layout::Marca {
                coluna: 0, caminho: "Sul".into(), recolhido: true,
            }],
        };
        assert_eq!(mapa_json(&[info], 1), json!({
            "rotulos":1, "linhas":[["g",2,[[0,"Sul",true]]]]
        }));
    }

    #[test]
    fn indices_dos_calculados_sao_negativos_para_compatibilidade_typescript() {
        let mut app = App::default();
        app.workbook.editar(0, |s| {
            s.set_cell(0,0,"A");s.set_cell(0,1,"B");
            s.set_cell(1,0,"10");s.set_cell(1,1,"20");
        });
        app.fim_selecao=(1,1);
        app.evento_dinamica(Evento::Abrir);
        app.evento_dinamica(Evento::NomeCalculado("SomaAB".into()));
        app.evento_dinamica(Evento::FormulaCalculado("A + B".into()));
        app.evento_dinamica(Evento::AdicionarCalculado);
        let e = app.dinamica.as_ref().unwrap();
        let mut meta = json!({});
        gravar_campos(&mut meta,e);
        assert!(meta["valores"].as_array().unwrap().iter().any(|v| v["col"] == -1));
    }

    #[test]
    fn configuracao_antiga_tem_layout_tabular_sem_filtros() {
        let (layout, repetir, subtotais, recolhidos, filtros) = opcoes_de_json(&json!({}));
        assert_eq!(layout, Layout::Tabular);
        assert!(repetir);
        assert!(!subtotais);
        assert!(recolhidos.is_empty() && filtros.is_empty());
    }
}
