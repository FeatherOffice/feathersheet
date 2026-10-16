//! Galeria inicial, editor com prévia e gráficos persistidos na aba.
use super::*;
use serde_json::json;

fn cartao<'a>(conteudo: impl Into<Element<'a, Msg>>) -> iced::widget::Container<'a, Msg> {
    container(conteudo).padding(18).style(|_| container::Style {
        background: Some(iced::Background::Color(feather_ui::tema::COR_PAINEL())),
        text_color: Some(feather_ui::tema::COR_TEXTO()),
        border: iced::Border {
            color: feather_ui::tema::COR_BORDA(),
            width: 1.,
            radius: 12.0.into(),
        },
        ..Default::default()
    })
}
/// Renderizador único de embeds: obtém os números da engine colunar do
/// FeatherSheet e só aplica estilo/desenho no Charts compartilhado.
/// Não escreve na aba, não armazena caches serializados e não agrega.
/// Quadro no lugar do gráfico que o motor não desenha.
fn previa_indisponivel() -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"480\" height=\"200\" viewBox=\"0 0 480 200\">\
         <rect x=\"0.5\" y=\"0.5\" width=\"479\" height=\"199\" fill=\"#ffffff\" stroke=\"#dadce0\"/>\
         <text x=\"240\" y=\"104\" text-anchor=\"middle\" font-size=\"14\" fill=\"#5f6368\" \
         font-family=\"Noto Sans, DejaVu Sans, sans-serif\">{}</text></svg>",
        "Prévia indisponível para esses dados ou tipo."
    )
}

pub(super) fn svg_do_grafico_salvo(
    sheet: &feathersheet_core::Sheet,
    valor: &serde_json::Value,
) -> Option<String> {
    let mut config = serde_json::from_value::<ChartConfig>(valor.clone()).ok()?;
    let linhas = u64::from(config.range.r1.checked_sub(config.range.r0)?) + 1;
    let colunas = u64::from(config.range.c1.checked_sub(config.range.c0)?) + 1;
    if linhas.checked_mul(colunas)? > 200_000 {
        return None;
    }
    // O extractor de pizza antigo pode reduzir a primeira série antes
    // de o painel escolher outra. A extração como colunas preserva TODAS
    // as séries; o Charts só recebe a série selecionada no passo seguinte.
    let pilha = super::empilhamento(config.tipo);
    if let Some((base, _)) = pilha {
        config.tipo = base;
    }
    let pizza = config.tipo == ChartType::Pie;
    if pizza {
        config.tipo = ChartType::Column;
    }
    // Desenha os tipos especializados pelo mesmo motor Rust da prévia.
    if !matches!(config.tipo,
        ChartType::Column | ChartType::Bar | ChartType::Line |
        ChartType::Area | ChartType::StepLine | ChartType::Scatter
    ) && !pizza {
        let aparencia: Aparencia =
            serde_json::from_value(valor["_featherNative"].clone()).unwrap_or_default();
        if aparencia.nomes_origem.is_empty() {
            return feathersheet_io::grafico_migrado::renderizar_sheet(
                sheet, &config, 800., 450.
            ).ok();
        }
        let mut dados = feathersheet_core::grafico::chart::extract_chart_data(sheet, &config);
        super::renomear_series_dados(&mut dados, &config.style.hidden_series, &aparencia);
        return feathersheet_io::grafico_migrado::renderizar_dados(&config, &dados, 800., 450.).ok();
    }
    let mut contrato = if valor["_excel"].is_object() {
        // Gráfico do Excel: célula vazia na série é lacuna lá; o motor comum
        // recusa lacuna, então vale 0 (categoria e cabeçalho ficam como estão).
        use feathersheet_core::celulas::values::Value as V;
        let (r0, c0) = (config.range.r0, config.range.c0);
        let dados = feathersheet_core::grafico::chart::extract_chart_data_with(&|r, c| match sheet.get_value(r, c) {
            V::Empty if r > r0 && c > c0 => V::Number(0.0),
            v => v,
        }, &config);
        feathersheet_io::grafico_compartilhado::preparar_grafico(&config, dados).ok()?
    } else {
        feathersheet_io::grafico_compartilhado::extrair_grafico(sheet, &config).ok()?
    };
    let aparencia: Aparencia =
        serde_json::from_value(valor["_featherNative"].clone()).unwrap_or_default();
    super::aplicar_nomes_origem(&mut contrato, &aparencia);
    if pizza {
        contrato.grafico.tipo = Tipo::Pizza;
        let indice = valor["_featherPieSeries"]
            .as_str()
            .and_then(|s| s.split(" — ").next())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1)
            .saturating_sub(1);
        let escolhida = contrato.grafico.series.get(indice)?.clone();
        contrato.grafico.series = vec![escolhida];
    }
    let nativa = valor["_featherNative"].is_object();
    let aparencia = match pilha {
        Some((_, emp)) => super::aparencia_empilhada(&aparencia, emp),
        None => aparencia,
    };
    if nativa {
        // Reaplica cores primárias, overrides e nomes EXATAMENTE como
        // a prévia. Sem essa chamada, editar um gráfico salvo alterava
        // a aparência durante o redraw, apesar da configuração persistida.
        aplicar_aparencia_grafico(&mut contrato.grafico, &aparencia).ok()?;
    }
    // Estilo novo (painel Formatar gráfico): desenhado no tamanho do quadro,
    // com fundo, cores e textos do próprio estilo (o do Excel já foi convertido).
    if aparencia.estilo.is_some() {
        let medida = |k: &str, padrao: f64| valor["position"][k].as_f64().unwrap_or(padrao).clamp(120., 3200.) as f32;
        return preparar_svg_em(&contrato.grafico, &aparencia, medida("width", 480.), medida("height", 300.)).ok();
    }
    // Documentos antigos sem _featherNative preservam as cores já
    // resolvidas pelo adaptador a partir de ChartConfig.style.series_colors.
    // Gráfico do `.xlsx`: desenhado no tamanho do quadro (letra em px reais)
    // e vestido com o fundo, o contorno e as cores de texto do Excel.
    if let Some(excel) = valor["_excel"].as_object() {
        let medida = |k: &str, padrao: f64| valor["position"][k].as_f64().unwrap_or(padrao).clamp(120., 3200.) as f32;
        let (w, h) = (medida("width", 480.), medida("height", 320.));
        let svg = preparar_svg_em(&contrato.grafico, &aparencia, w, h).ok()?;
        return Some(super::excel::vestir(svg, excel, w, h));
    }
    preparar_svg(&contrato.grafico, &aparencia).ok()
}

impl App {
    pub(in crate::app) fn sobrepor_galeria_graficos<'a>(
        &'a self,
        base: Element<'a, Msg>,
    ) -> Element<'a, Msg> {
        if !self.graficos.seletor_aberto {
            return base;
        }
        // A galeria abre imediatamente abaixo do botão Gráfico, e NÃO
        // abaixo de toda a Ribbon (que inclui legenda e pista de rolagem).
        // O X do clique continua congelado para o popup não seguir o mouse.
        let topo = feather_ui::cromo::BARRA_ALTURA
            + (if self.config.barra_propria { 2.0 } else { 1.0 });
        let abas = 44.0 + 8.0; // scroll das abas + padding vertical
        let abaixo_botao = crate::geometria_overlay::abaixo_botao_grafico(
            topo, abas, self.config.faixa_recolhida,
        );
        let Some(pos) = crate::geometria_overlay::galeria_graficos(
            self.graficos.ancora_seletor_x,
            self.tamanho_principal.width,
            self.tamanho_principal.height,
            abaixo_botao,
            seletor_tipo::LARGURA_POPOVER,
            seletor_tipo::ALTURA_POPOVER,
        ) else { return base; };
        let fora = iced::widget::mouse_area(Space::new(Length::Fill, Length::Fill))
            .on_press(Msg::Grafico(Evento::FecharSeletor));
        let painel = cartao(seletor_tipo::painel(self.graficos.tipo))
            .padding(6)
            .width(Length::Fixed(pos.largura))
            .height(Length::Fixed(pos.altura));
        iced::widget::stack![
            base,
            fora,
            iced::widget::column![
                Space::with_height(Length::Fixed(pos.y)),
                iced::widget::row![
                    Space::with_width(Length::Fixed(pos.x)),
                    painel
                ]
            ]
        ]
        .height(Length::Fill)
        .into()
    }
    /// O JSON do gráfico com o que está no editor. Mesclado no gravado
    /// (`graficos_salvos::mesclado`), preserva `_excel`, `sourceSheet` e o resto.
    fn json_do_editor(&self, id: &str) -> Result<serde_json::Value, String> {
        let g = &self.graficos;
        let r = parse_range_ref(&g.intervalo).map(|r| r.normalized())
            .ok_or("Intervalo inválido. Use A1:C10.")?;
        let tipo = if g.tipo == TipoUi::Pizza {
            "pie".to_owned()
        } else {
            serde_json::to_value(g.tipo.motor()).ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_else(|| "column".to_owned())
        };
        let a = &g.aparencia;
        let dimensao = |s: &str| {
            s.parse::<f32>()
                .ok()
                .filter(|n| n.is_finite() && (100.0..=3200.0).contains(n))
        };
        let mut valor = json!({"id":id,"type":tipo,"title":g.titulo,"range":r,"seriesInColumns":g.colunas,"hasHeaders":g.cabecalho,"hasCategoryLabels":g.categorias,"aggregate":g.agrupar,"aggregateFn":"sum","style":{"showTitle":a.titulo,"subtitle":a.subtitulo,"legendPosition":if a.legenda{"bottom"}else{"none"},"dataLabels":a.valores,"seriesColors":{"0":a.cor}},"_featherNative":a,"_featherPieSeries":g.serie});
        for (i, c) in &a.cores {
            valor["style"]["seriesColors"][i.to_string()] = json!(c);
        }
        // Aba Configurar: coluna das categorias, séries ocultas e a conta de cada uma.
        valor["categoryColumnIndex"] = json!(g.coluna_categoria);
        valor["style"]["hiddenSeries"] = json!(g.ocultas);
        valor["aggregateBySeries"] = serde_json::to_value(&g.contas).unwrap_or_default();
        if let Some(e) = &a.estilo {
            // O gravador `.xlsx` lê as cores daqui: as que se veem (tema/estilo).
            for i in 0..g.series.len() {
                let c = super::painel::cor_resolvida_serie(e, &g.painel.cores_base, i);
                valor["style"]["seriesColors"][i.to_string()] = json!(c.texto());
            }
        }
        let salvo = self.indice_aba_grafico()
            .and_then(|i| self.workbook.aba(i))
            .and_then(|aba| feathersheet_io::graficos_salvos::listar(&aba.sheet).ok())
            .and_then(|l| l.into_iter().find(|v| v["id"].as_str() == Some(id)));
        let mut posicao = salvo.as_ref().map(|v| v["position"].clone())
            .filter(serde_json::Value::is_object)
            .unwrap_or_else(|| json!({"x":80,"y":80,"width":480,"height":300}));
        // Medida fora da faixa (digitando) não estraga o quadro gravado.
        if let (Some(largura), Some(altura)) = (dimensao(&a.largura), dimensao(&a.altura)) {
            posicao["width"] = json!(largura);
            posicao["height"] = json!(altura);
        }
        valor["position"] = posicao;
        Ok(valor)
    }

    /// Índice da aba onde o gráfico fica (a dos dados, se não houver outra).
    pub(super) fn indice_aba_grafico(&self) -> Option<usize> {
        let alvo = self.graficos.aba_grafico.or(self.graficos.aba_id)?;
        (0..self.workbook.len()).find(|&i| self.workbook.aba(i).is_some_and(|a| a.id == alvo))
    }

    /// Grava o editor no gráfico da planilha AO VIVO: cada edição é um passo
    /// de Ctrl+Z; letras seguidas no mesmo campo (`campo`) juntam num passo.
    pub(super) fn salvar_grafico_ao_vivo(&mut self, campo: Option<&'static str>) {
        let Some(id) = self.graficos.id.clone() else { return };
        let juntar = campo.is_some() && campo == self.graficos.ultimo_campo;
        self.graficos.ultimo_campo = campo;
        let Ok(valor) = self.json_do_editor(&id) else { return };
        let Some(indice) = self.indice_aba_grafico() else { return };
        let Some(aba) = self.workbook.aba_mut(indice) else { return };
        match feathersheet_io::graficos_salvos::mesclado(&aba.sheet, &id, valor) {
            Ok(novo) => {
                if aba.sheet.alterar_graficos(Some(novo), juntar) {
                    self.alterado = true;
                }
            }
            Err(e) => self.graficos.mensagem = e,
        }
    }

    pub(super) fn inserir_grafico(&mut self) -> Task<Msg> {
        self.atualizar_grafico();
        if self.graficos.svg.is_none() && self.graficos.id.is_none() {
            return Task::none();
        }
        let id = self.graficos.id.clone().unwrap_or_else(|| {
            format!(
                "native-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
        });
        let valor = match self.json_do_editor(&id) {
            Ok(v) => v,
            Err(e) => {
                self.graficos.mensagem = e;
                return Task::none();
            }
        };
        let Some(indice) = self.indice_aba_grafico() else {
            self.graficos.mensagem = "A aba de origem foi removida.".into();
            return Task::none();
        };
        let aba = self.workbook.aba_mut(indice).unwrap();
        match feathersheet_io::graficos_salvos::mesclado(&aba.sheet, &id, valor) {
            Ok(novo) => { aba.sheet.alterar_graficos(Some(novo), false); }
            Err(e) => {
                self.graficos.mensagem = e;
                return Task::none();
            }
        }
        self.graficos.ultimo_campo = None;
        self.alterado = true;
        self.status = "Gráfico salvo na planilha.".into();
        self.graficos.id = Some(id);
        self.graficos.aberto = true;
        self.atualizar_grafico();
        Task::none()
    }
    /// Depois de Ctrl+Z/Ctrl+Y com o painel aberto: o painel volta a mostrar
    /// o gráfico gravado (ou fecha, se o gráfico deixou de existir).
    pub(in crate::app) fn recarregar_grafico_aberto(&mut self) {
        if !self.graficos.aberto { return; }
        let Some(id) = self.graficos.id.clone() else { return };
        let existe = self.workbook.aba(self.folha)
            .and_then(|a| feathersheet_io::graficos_salvos::listar(&a.sheet).ok())
            .is_some_and(|l| l.iter().any(|v| v["id"].as_str() == Some(id.as_str())));
        if existe {
            let antes = std::mem::take(&mut self.graficos.painel);
            let _ = self.editar_grafico_salvo(&id);
            let p = &mut self.graficos.painel;
            (p.grupo, p.eixo, p.serie, p.ponto, p.area) = (antes.grupo, antes.eixo, antes.serie, antes.ponto, antes.area);
        } else {
            self.graficos.aberto = false;
            self.graficos.id = None;
            self.graficos.selecionado = None;
        }
    }

    pub(super) fn editar_grafico_salvo(&mut self, id: &str) -> Task<Msg> {
        let Some(aba) = self.workbook.aba(self.folha) else {
            return Task::none();
        };
        let Ok(lista) = feathersheet_io::graficos_salvos::listar(&aba.sheet) else {
            return Task::none();
        };
        let Some(v) = lista.iter().find(|v| v["id"].as_str() == Some(id)) else {
            return Task::none();
        };
        let Ok(c) = serde_json::from_value::<ChartConfig>(v.clone()) else {
            return Task::none();
        };
        let tipo = match c.tipo {
            ChartType::Column => TipoUi::Colunas,
            ChartType::ColumnStacked => TipoUi::ColunasEmpilhadas,
            ChartType::ColumnStacked100 => TipoUi::Colunas100,
            ChartType::Bar => TipoUi::Barras,
            ChartType::BarStacked => TipoUi::BarrasEmpilhadas,
            ChartType::BarStacked100 => TipoUi::Barras100,
            ChartType::Line => TipoUi::Linhas,
            ChartType::Line3d => TipoUi::Linha3d,
            ChartType::StepLine => TipoUi::Degraus,
            ChartType::StepLineStacked => TipoUi::DegrausEmpilhados,
            ChartType::StepLineStacked100 => TipoUi::Degraus100,
            ChartType::Area => TipoUi::Area,
            ChartType::StepArea => TipoUi::AreaDegraus,
            ChartType::StepAreaStacked => TipoUi::AreaDegrausEmpilhada,
            ChartType::StepAreaStacked100 => TipoUi::AreaDegraus100,
            ChartType::Pie => TipoUi::Pizza,
            ChartType::Scatter => TipoUi::Dispersao,
            ChartType::Bubble => TipoUi::Bolhas,
            ChartType::Combo => TipoUi::Combinado,
            ChartType::Heatmap => TipoUi::MapaCalor,
            ChartType::Waterfall => TipoUi::Cascata,
            ChartType::Histogram => TipoUi::Histograma,
            ChartType::Radar => TipoUi::Radar,
            ChartType::Candlestick => TipoUi::Candlestick,
            ChartType::Gantt => TipoUi::Gantt,
            ChartType::Outro => {
                self.status = "Tipo de gráfico não reconhecido.".into();
                return Task::none();
            }
        };
        // A alça atualiza position no .fshz; o editor deve mostrar as
        // dimensões persistidas mesmo quando _featherNative é legado.
        let mut aparencia: Aparencia =
            serde_json::from_value(v["_featherNative"].clone()).unwrap_or_default();
        for (campo, destino) in [
            ("width", &mut aparencia.largura),
            ("height", &mut aparencia.altura),
        ] {
            if let Some(n) = v["position"][campo].as_f64().filter(|n| n.is_finite()) {
                *destino = format!("{n:.0}");
            }
        }
        // Gráfico do `.xlsx` com os dados em outra aba (`sourceSheet`): a
        // prévia lê de lá; o gráfico continua gravado na aba onde aparece.
        let aba_dados = v["sourceSheet"].as_str()
            .and_then(|n| self.workbook.indice_por_nome(n))
            .and_then(|i| self.workbook.aba(i))
            .map_or(aba.id, |a| a.id);
        let copiado = self.graficos.copiado.take();
        self.graficos = Estado {
            copiado,
            aberto: true,
            id: Some(id.into()),
            aba_grafico: Some(aba.id),
            intervalo: format_ref(&feathersheet_core::celulas::refs::Rect::new(
                c.range.r0, c.range.c0, c.range.r1, c.range.c1,
            )),
            titulo: c.title,
            tipo,
            cabecalho: c.has_headers,
            categorias: c.has_category_labels,
            colunas: c.series_in_columns,
            agrupar: c.aggregate,
            coluna_categoria: c.category_column_index.unwrap_or(0),
            ocultas: c.style.hidden_series.clone(),
            contas: c.aggregate_by_series.clone(),
            aba_id: Some(aba_dados),
            aparencia,
            serie: v["_featherPieSeries"].as_str().map(str::to_string),
            ..Default::default()
        };
        self.graficos.painel.excel = v["_excel"].as_object().cloned();
        self.atualizar_grafico();
        Task::none()
    }
    /// Os gráficos da aba ativa para a camada de objetos desenhar:
    /// `(z, retângulo, svg)`. Os do `.xlsx` vêm na ordem do desenho do
    /// Excel (`_excel.z`); os criados no app ficam por cima de tudo.
    pub(in crate::app) fn graficos_da_camada(&self) -> Vec<(u64, feathersheet_io::objetos_aba::Retangulo, String)> {
        let Some(aba) = self.workbook.aba(self.folha) else { return Vec::new() };
        let Ok(lista) = feathersheet_io::graficos_salvos::listar(&aba.sheet) else { return Vec::new() };
        let mut saida = Vec::new();
        for v in lista {
            let Some(quadro) = Quadro::da_posicao(&v["position"]) else { continue };
            let dados = v["sourceSheet"].as_str()
                .and_then(|n| self.workbook.indice_por_nome(n))
                .and_then(|i| self.workbook.aba(i))
                .map_or(&aba.sheet, |a| &a.sheet);
            // O editor grava ao vivo: o gráfico aberto se desenha do mesmo
            // jeito que fechado (antes, a prévia do editor lia a aba errada no
            // gráfico do `.xlsx` e ele sumia ao abrir "Editar gráfico…").
            let svg = svg_do_grafico_salvo(dados, &v).unwrap_or_else(previa_indisponivel);
            let ret = feathersheet_io::objetos_aba::Retangulo::novo(
                quadro.x as f32, quadro.y as f32, quadro.largura as f32, quadro.altura as f32,
            );
            saida.push((v["_excel"]["z"].as_u64().unwrap_or(u64::MAX), ret, svg));
        }
        saida
    }

    pub(in crate::app) fn com_graficos_salvos<'a>(
        &'a self,
        grade: Element<'a, Msg>,
    ) -> Element<'a, Msg> {
        if self.pendente.is_some() || self.contexto.is_some() {
            return grade;
        }
        let Some(aba) = self.workbook.aba(self.folha) else {
            return grade;
        };
        let Ok(lista) = feathersheet_io::graficos_salvos::listar(&aba.sheet) else {
            return grade;
        };

        // A posição é absoluta na FOLHA, e não na janela. A grade Iced possui
        // cabeçalhos próprios, scroll por índices de célula e zoom separado.
        let zoom = (self.config.zoom as f64 / 100.).max(0.01);
        let scroll = posicionamento::scroll_em_pixels(
            &aba.sheet, self.linha_inicio, self.coluna_inicio,
        );
        let mut graficos = iced::widget::stack![
            Space::with_width(Length::Fill).height(Length::Fill)
        ].width(Length::Fill).height(Length::Fill);

        for v in lista {
            let Some(id) = v["id"].as_str().map(str::to_owned) else {
                continue;
            };
            let Some(quadro) = Quadro::da_posicao(&v["position"]) else {
                continue;
            };

            let (x, y) = posicionamento::origem_tela(quadro, scroll, zoom);
            // Widget Iced 0.13 não posiciona crianças em coordenadas negativas.
            // Ao chegar à borda superior/esquerda, ocultar o embed em vez de
            // deslocá-lo artificialmente ou pintá-lo sobre os cabeçalhos.
            if x < 0. || y < 0. || x > 100_000. || y > 100_000. {
                continue;
            }
            let largura = (quadro.largura * zoom).clamp(120., 3200.) as f32;
            let altura = (quadro.altura * zoom).clamp(100., 3200.) as f32;

            // O desenho de todo gráfico sai na camada de objetos
            // (`graficos_da_camada`, intercalado com as formas e recortado
            // pela grade); aqui fica só a área de clique: selecionar e
            // arrastar pelo corpo, duplo clique edita, botão direito abre o menu.
            let selecionado = self.graficos.selecionado.as_deref() == Some(id.as_str())
                || (self.graficos.aberto && self.graficos.id.as_deref() == Some(id.as_str()));
            // A roda sobre o gráfico rola a planilha (Ctrl+roda dá zoom),
            // como no Excel: a área de clique esconde o cursor da grade.
            let ctrl = self.ctrl_pressionado;
            let item = iced::widget::mouse_area(Space::new(Length::Fill, Length::Fill))
                .on_press(Msg::Grafico(Evento::Arrastar(id.clone())))
                .on_right_press(Msg::Grafico(Evento::Contexto(id.clone())))
                .on_scroll(move |d| {
                    let y = match d {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 40.0,
                    };
                    if ctrl && y.abs() >= 0.1 {
                        Msg::Roda(y.signum() as i32)
                    } else {
                        crate::app::botoes_macro::rolar_pela_roda(d)
                    }
                })
                .interaction(mouse::Interaction::Grab);

            // Conteúdo opaco e CLIPADO dentro da própria borda do gráfico:
            // SVG não deve ultrapassar o retângulo, nem consumir os handles.
            let painel = container(item)
                .width(Length::Fixed(largura))
                .height(Length::Fixed(altura))
                .style(move |_| container::Style {
                    border: iced::Border {
                        color: feather_ui::tema::COR_AZUL_ACENTO(),
                        width: if selecionado { 2. } else { 0. },
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                });

            let conteudo: Element<'_, Msg> = if selecionado {
                // Oito quadradinhos no estilo Google Sheets, sobrepostos às
                // bordas sem adicionar largura/altura ao gráfico no .fshz.
                let cantos = iced::widget::column![
                    iced::widget::row![
                        alca_grafico(&id, Alca::Noroeste),
                        Space::with_width(Length::Fill),
                        alca_grafico(&id, Alca::Norte),
                        Space::with_width(Length::Fill),
                        alca_grafico(&id, Alca::Nordeste),
                    ],
                    Space::with_height(Length::Fill),
                    iced::widget::row![
                        alca_grafico(&id, Alca::Oeste),
                        Space::with_width(Length::Fill),
                        alca_grafico(&id, Alca::Leste),
                    ],
                    Space::with_height(Length::Fill),
                    iced::widget::row![
                        alca_grafico(&id, Alca::Sudoeste),
                        Space::with_width(Length::Fill),
                        alca_grafico(&id, Alca::Sul),
                        Space::with_width(Length::Fill),
                        alca_grafico(&id, Alca::Sudeste),
                    ]
                ].width(Length::Fixed(largura)).height(Length::Fixed(altura));
                iced::widget::stack![painel, cantos].into()
            } else {
                painel.into()
            };
            graficos = graficos.push(
                iced::widget::column![
                    Space::with_height(Length::Fixed(y as f32)),
                    iced::widget::row![
                        Space::with_width(Length::Fixed(x as f32)),
                        conteudo
                    ],
                ]
            );
        }

        // A área dos gráficos NÃO inclui os cabeçalhos; a máscara impede que
        // o SVG e as alças invadam bordas da grade, barra de abas e editor.
        let celulas = container(graficos)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true);
        let cobertura = iced::widget::column![
            Space::with_height(Length::Fixed(
                crate::grade::CABECALHO_COLUNA * zoom as f32
            )),
            iced::widget::row![
                Space::with_width(Length::Fixed(
                    crate::grade::CABECALHO_LINHA * zoom as f32
                )),
                celulas
            ].width(Length::Fill).height(Length::Fill)
        ].width(Length::Fill).height(Length::Fill);
        let sobre_grade = container(iced::widget::stack![grade, cobertura])
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true);
        if self.graficos.aberto {
            iced::widget::row![sobre_grade, self.view_graficos()]
                .height(Length::Fill)
                .into()
        } else {
            sobre_grade.into()
        }
    }
}

/// Handle visível e clicável; com 8 pontos de captura independentes.
fn alca_grafico<'a>(id: &str, direcao: Alca) -> Element<'a, Msg> {
    let alvo = iced::widget::mouse_area(
        container(Space::with_width(Length::Fixed(10.)).height(Length::Fixed(10.)))
            .width(Length::Fixed(10.))
            .height(Length::Fixed(10.))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(feather_ui::tema::COR_PAINEL())),
                border: iced::Border {
                    color: feather_ui::tema::COR_AZUL_ACENTO(),
                    width: 1.5,
                    radius: 1.0.into(),
                },
                ..Default::default()
            })
    )
    .on_press(Msg::Grafico(Evento::Redimensionar(id.to_owned(), direcao)))
    .interaction(mouse::Interaction::ResizingDiagonallyDown);
    alvo.into()
}
