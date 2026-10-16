//! Desenho das ferramentas do código (14/10/2026): barra Localizar/Substituir,
//! calha com o número das linhas, destaque das ocorrências e busca do
//! Explorador. A lógica está em `macros/codigo.rs`.
//!
//! **Por que a calha fica alinhada:** o `text_editor` do Iced 0.13 não expõe a
//! rolagem. Então ele recebe a altura exata do texto (linhas × 18 px, linha de
//! altura fixa e sem quebra) e quem rola é um `scrollable` nosso, que leva a
//! calha e o editor juntos na mesma `row`: não há rolagem a sincronizar. A
//! rolagem horizontal fica num `scrollable` interno só do editor, para os
//! números não saírem da vista. Os números são três `text` (antes, a linha do
//! cursor, depois) com a mesma fonte, tamanho e altura de linha do editor.
use super::*;
use crate::app::macros::codigo::{
    self, EventoCodigo, Ocorrencia, ALTURA_LINHA, LARGURA_CARACTERE, MARGEM_H, MARGEM_V,
    TAMANHO_FONTE,
};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{column, stack, tooltip};
use iced::{Font, Padding};
use feather_ui::tema::{COR_AZUL_ACENTO, COR_AZUL_FUNDO, COR_PAINEL_FAIXA, COR_TEXTO, COR_TEXTO_MUTED};

fn ev(e: EventoCodigo) -> Msg { Msg::MacroVisual(EventoVisual::Codigo(e)) }

fn com_dica<'a>(conteudo: impl Into<Element<'a, Msg>>, dica: &str) -> Element<'a, Msg> {
    tooltip(conteudo, text(dica.to_owned()).size(11), tooltip::Position::Bottom)
        .gap(4).padding(6).style(feather_ui::tema::estilo_dica).into()
}

fn linha_fixa() -> LineHeight { LineHeight::Absolute(ALTURA_LINHA.into()) }

/// Texto invisível com a medida exata do código (para posicionar destaques).
fn medida<'a>(s: String) -> iced::widget::Text<'a> {
    text(s).font(Font::MONOSPACE).size(TAMANHO_FONTE).line_height(linha_fixa())
        .shaping(Shaping::Advanced).wrapping(Wrapping::None).color(Color::TRANSPARENT)
}

/// Contador "3 de 12" da barra.
pub(in crate::app) fn contador(achadas: &[Ocorrencia], atual: Option<(usize, usize)>, consulta: &str) -> String {
    if consulta.is_empty() { return String::new(); }
    if achadas.is_empty() { return "Nenhum resultado".into(); }
    let total = if achadas.len() >= 10_000 { "10.000+".to_string() } else { achadas.len().to_string() };
    match atual.and_then(|p| achadas.iter().position(|o| (o.linha, o.coluna) == p)) {
        Some(i) => format!("{} de {total}", i + 1),
        None => format!("{total} resultado(s)"),
    }
}

/// Barra compacta no topo do código: Localizar e, com Ctrl+H, Substituir.
fn barra_busca<'a>(app: &'a App, achadas: &[Ocorrencia]) -> Element<'a, Msg> {
    let c = &app.macro_visual.codigo;
    let estilo_campo = feather_ui::tema::estilo_entrada_suspensa;
    let alternar = com_dica(
        botao_icone_simples(if c.substituir { "chevron-down" } else { "chevron-right" },
            Some(ev(EventoCodigo::AlternarSubstituir))),
        if c.substituir { "Ocultar Substituir" } else { "Mostrar Substituir  Ctrl+H" },
    );
    let tem = !achadas.is_empty();
    let primeira = row![
        alternar,
        feather_ui::icones::icone("search", 15.0),
        text_input("Localizar no código", &c.consulta)
            .id(codigo::id_campo_busca())
            .on_input(|t| ev(EventoCodigo::Consulta(t)))
            .on_submit(ev(EventoCodigo::Enter))
            .size(12).padding([4, 7]).width(Length::Fixed(240.0)).style(estilo_campo),
        com_dica(botao("Maiúsculas", Some(ev(EventoCodigo::AlternarMaiusculas)), c.maiusculas),
            "Diferenciar maiúsculas e minúsculas"),
        com_dica(botao("Palavra inteira", Some(ev(EventoCodigo::AlternarPalavra)), c.palavra),
            "Só a palavra inteira (não acha \"total\" dentro de \"subtotal\")"),
        com_dica(botao("Todos os módulos", Some(ev(EventoCodigo::AlternarTodos)), c.todos_modulos),
            "Listar as ocorrências em todos os módulos"),
        text(contador(achadas, c.atual, &c.consulta)).size(11).color(COR_TEXTO_MUTED()),
        Space::with_width(Length::Fill),
        com_dica(botao_icone_simples("chevron-up", tem.then(|| ev(EventoCodigo::Anterior))),
            "Anterior  Shift+Enter / Shift+F3"),
        com_dica(botao_icone_simples("chevron-down", tem.then(|| ev(EventoCodigo::Proximo))),
            "Próxima  Enter / F3"),
        com_dica(botao_icone_simples("x", Some(ev(EventoCodigo::Fechar))), "Fechar  Esc"),
    ].spacing(5).align_y(Alignment::Center);
    let mut linhas = column![primeira].spacing(4);
    if c.substituir {
        linhas = linhas.push(row![
            Space::with_width(Length::Fixed(29.0)),
            feather_ui::icones::icone("edit", 15.0),
            text_input("Substituir por", &c.troca)
                .id(codigo::id_campo_troca())
                .on_input(|t| ev(EventoCodigo::Troca(t)))
                .on_submit(ev(EventoCodigo::Substituir))
                .size(12).padding([4, 7]).width(Length::Fixed(240.0)).style(estilo_campo),
            com_dica(botao("Substituir", tem.then(|| ev(EventoCodigo::Substituir)), false),
                "Substituir a ocorrência atual e ir para a próxima  Enter"),
            com_dica(botao_icone("Substituir todos", "checks", tem.then(|| ev(EventoCodigo::SubstituirTodos)), false),
                "Substituir todas de uma vez (um só Ctrl+Z desfaz)"),
            text(c.aviso.clone()).size(11).color(COR_TEXTO_MUTED()),
        ].spacing(5).align_y(Alignment::Center));
    }
    container(linhas).padding([5, 8]).width(Length::Fill)
        .style(estilo_cabecalho(app.config.escuro)).into()
}

/// Lista "em todos os módulos": módulo:linha clicável e o trecho da linha.
fn resultados_todos<'a>(app: &'a App) -> Element<'a, Msg> {
    let resultados = app.ocorrencias_em_todos();
    let mut lista = column![].spacing(1);
    if resultados.is_empty() {
        lista = lista.push(text(if app.macro_visual.codigo.consulta.is_empty() {
            "Digite o que procurar."
        } else { "Nenhum módulo contém esse texto." }).size(11).color(COR_TEXTO_MUTED()));
    }
    for (modulo, o, trecho) in resultados {
        let rotulo = format!("{modulo}:{}", o.linha + 1);
        lista = lista.push(button(row![
            feather_ui::icones::icone("file-code", 13.0),
            text(rotulo).size(11).font(Font::MONOSPACE).width(Length::Fixed(150.0)),
            text(trecho).size(11).font(Font::MONOSPACE).color(COR_TEXTO_MUTED()),
        ].spacing(6).align_y(Alignment::Center))
            .padding([3, 8]).width(Length::Fill)
            .on_press(ev(EventoCodigo::IrPara(modulo, o.linha, o.coluna)))
            .style(estilo_botao_ide(false)));
    }
    container(scrollable(lista).height(Length::Shrink))
        .max_height(150.0).padding([4, 6]).width(Length::Fill).into()
}

/// Destaques das ocorrências por cima do editor (translúcidos): o
/// `text_editor` só desenha a seleção quando tem o foco, e o foco fica no
/// campo de busca enquanto se procura.
///
/// Só entram as ocorrências inteiras à vista: a camada de cima do `stack`
/// abre um `with_layer` com o recorte da rolagem horizontal, que no Iced 0.13
/// é o documento inteiro (não o pedaço visível), e o destaque vazaria por
/// cima da barra de busca e do rodapé.
fn destaques<'a>(texto: &str, achadas: &[Ocorrencia], atual: Option<(usize, usize)>,
    visiveis: std::ops::Range<usize>, colunas: std::ops::Range<usize>) -> Element<'a, Msg> {
    let filtradas: Vec<Ocorrencia> = achadas.iter().copied()
        .filter(|o| visiveis.contains(&o.linha) && o.coluna >= colunas.start
            && o.coluna + o.tamanho <= colunas.end)
        .collect();
    let achadas = &filtradas[..];
    let linhas: Vec<&str> = texto.split('\n').collect();
    let mut col = column![].padding(Padding { top: MARGEM_V, left: MARGEM_H, right: 0.0, bottom: 0.0 });
    let mut proxima = 0usize;
    let mut i = 0;
    // Teto: destacar mais que isso só pesa o desenho.
    let achadas = &achadas[..achadas.len().min(2_000)];
    while i < achadas.len() {
        let l = achadas[i].linha;
        if l > proxima {
            col = col.push(Space::with_height(Length::Fixed((l - proxima) as f32 * ALTURA_LINHA)));
        }
        let cs: Vec<char> = linhas.get(l).map_or(Vec::new(), |t| t.chars().collect());
        let mut fila = row![];
        let mut pos = 0;
        while i < achadas.len() && achadas[i].linha == l {
            let o = achadas[i];
            let fim = (o.coluna + o.tamanho).min(cs.len());
            if o.coluna > pos {
                fila = fila.push(medida(cs[pos..o.coluna].iter().collect()));
            }
            let e_atual = atual == Some((o.linha, o.coluna));
            let mut cor = COR_AZUL_ACENTO();
            cor.a = if e_atual { 0.42 } else { 0.16 };
            fila = fila.push(container(medida(cs[o.coluna.min(fim)..fim].iter().collect()))
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(cor)),
                    border: Border { color: if e_atual { COR_AZUL_ACENTO() } else { Color::TRANSPARENT },
                        width: if e_atual { 1.0 } else { 0.0 }, radius: 2.0.into() },
                    ..Default::default()
                }));
            pos = fim;
            i += 1;
        }
        col = col.push(fila);
        proxima = l + 1;
    }
    col.into()
}

/// Linhas inteiramente à vista na rolagem vertical (deslocamento, altura).
pub(in crate::app) fn linhas_visiveis(rolagem: Option<(f32, f32)>, altura_texto: f32) -> std::ops::Range<usize> {
    match rolagem {
        // Sem rolagem conhecida, ou o texto cabe: tudo está à vista.
        None => 0..usize::MAX,
        Some((_, altura)) if altura_texto <= altura => 0..usize::MAX,
        Some((desl, altura)) => {
            let primeira = ((desl - MARGEM_V).max(0.0) / ALTURA_LINHA).ceil() as usize;
            let fim = ((desl + altura - MARGEM_V).max(0.0) / ALTURA_LINHA).floor() as usize;
            primeira..fim.max(primeira)
        }
    }
}

/// Colunas inteiramente à vista na rolagem horizontal (deslocamento, largura).
pub(in crate::app) fn colunas_visiveis(rolagem: Option<(f32, f32)>, largura_texto: f32) -> std::ops::Range<usize> {
    match rolagem {
        None => 0..usize::MAX,
        Some((_, largura)) if largura_texto <= largura => 0..usize::MAX,
        Some((desl, largura)) => {
            let primeira = ((desl - MARGEM_H).max(0.0) / LARGURA_CARACTERE).ceil() as usize;
            let fim = ((desl + largura - MARGEM_H).max(0.0) / LARGURA_CARACTERE).floor() as usize;
            primeira..fim.max(primeira)
        }
    }
}

/// Calha com o número das linhas; o número da linha do cursor em destaque.
fn calha<'a>(total: usize, linha_cursor: usize, largura: f32, altura: f32) -> Element<'a, Msg> {
    let numeros = |de: usize, ate: usize| -> String {
        (de..=ate).map(|n| n.to_string()).collect::<Vec<_>>().join("\n")
    };
    let numero = |s: String, cor: Color| {
        text(s).font(Font::MONOSPACE).size(TAMANHO_FONTE).line_height(linha_fixa())
            .wrapping(Wrapping::None).width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right).color(cor)
    };
    let mut col = column![Space::with_height(Length::Fixed(MARGEM_V))];
    if linha_cursor > 1 {
        col = col.push(numero(numeros(1, linha_cursor - 1), COR_TEXTO_MUTED()));
    }
    col = col.push(container(numero(linha_cursor.to_string(), COR_TEXTO()))
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(COR_AZUL_FUNDO())),
            border: Border { radius: 3.0.into(), ..Border::default() },
            ..Default::default()
        }));
    if linha_cursor < total {
        col = col.push(numero(numeros(linha_cursor + 1, total), COR_TEXTO_MUTED()));
    }
    container(col.padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 4.0 }))
        .width(Length::Fixed(largura)).height(Length::Fixed(altura))
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(COR_PAINEL_FAIXA())),
            ..Default::default()
        }).into()
}

/// Largura da calha para `total` linhas (no mínimo dois dígitos).
pub(in crate::app) fn largura_calha(total: usize) -> f32 {
    total.to_string().len().max(2) as f32 * LARGURA_CARACTERE + 16.0
}

/// Código com calha de números, destaques e as duas rolagens.
fn area_codigo<'a>(app: &'a App, achadas: &[Ocorrencia]) -> Element<'a, Msg> {
    let escuro = app.config.escuro;
    let c = &app.macro_visual.codigo;
    let texto = app.macro_codigo.text();
    let total = app.macro_codigo.line_count().max(1);
    let (linha_cursor, _) = codigo::linha_coluna(&app.macro_codigo);
    let calha_px = largura_calha(total);
    // Largura: a linha mais comprida ou a área visível (estimada pela janela).
    let maior = texto.split('\n')
        .map(|l| l.chars().map(|ch| if ch == '\t' { 8 } else { 1 }).sum::<usize>())
        .max().unwrap_or(0);
    let lateral = if app.macro_visual.explorador || app.macro_visual.propriedades { 288.0 } else { 0.0 };
    let visivel_x = c.janela.width - 16.0 - lateral - 14.0 - calha_px - 14.0;
    let largura = (maior as f32 * LARGURA_CARACTERE + 2.0 * MARGEM_H + 32.0).max(visivel_x).max(240.0);
    // Altura: a do texto, ou a da área visível (para o clique abaixo do fim
    // ainda cair no editor). Estimada pela janela, um pouco para menos.
    let console = if app.macro_visual.console {
        if app.macro_visual.painel == PAINEL_HISTORICO { 297.0 } else { 171.0 }
    } else { 0.0 };
    let barra_busca = match (c.busca_aberta, c.substituir) { (false, _) => 0.0, (true, true) => 76.0, (true, false) => 44.0 };
    let lista_todos = if c.busca_aberta && c.todos_modulos { 160.0 } else { 0.0 };
    let busca = barra_busca + lista_todos;
    let visivel_y = c.janela.height - 380.0 - console - busca;
    let altura_texto = total as f32 * ALTURA_LINHA + 2.0 * MARGEM_V;
    let altura = altura_texto.max(visivel_y);
    let editor = text_editor(&app.macro_codigo)
        .on_action(Msg::MacroEditar)
        .highlight("js", if escuro {
            iced::highlighter::Theme::Base16Ocean
        } else {
            iced::highlighter::Theme::InspiredGitHub
        })
        .font(Font::MONOSPACE)
        .size(TAMANHO_FONTE)
        .line_height(linha_fixa())
        .wrapping(Wrapping::None)
        .width(largura)
        .height(Length::Fixed(altura))
        .padding(Padding { top: MARGEM_V, bottom: MARGEM_V, left: MARGEM_H, right: MARGEM_H })
        .style(move |_, _| text_editor::Style {
            background: Background::Color(cor_fundo(escuro)),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
            icon: cor_secundaria(escuro),
            placeholder: cor_secundaria(escuro),
            value: cor_texto(escuro),
            selection: cor_selecao_codigo(escuro),
        });
    let mut camadas = stack![editor];
    if !achadas.is_empty() {
        let visiveis = linhas_visiveis(c.rolagem_v, altura);
        let colunas = colunas_visiveis(c.rolagem_h, largura);
        camadas = camadas.push(destaques(&texto, achadas, c.atual, visiveis, colunas));
    }
    // O contêiner com Id deixa o foco voltar ao editor ao fechar a busca.
    let editor = container(camadas).id(iced::widget::container::Id::new(codigo::ID_AREA_CODIGO));
    let horizontal = scrollable(editor)
        .id(codigo::id_rolagem_h())
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::new(),
            horizontal: scrollable::Scrollbar::new(),
        })
        .on_scroll(|v| ev(EventoCodigo::RolouH(v)))
        .width(Length::Fill);
    let conteudo = row![calha(total, linha_cursor, calha_px, altura), horizontal];
    scrollable(conteudo)
        .id(codigo::id_rolagem())
        .on_scroll(|v| ev(EventoCodigo::Rolou(v)))
        .height(Length::Fill).width(Length::Fill).into()
}

/// Área central do módulo: cabeçalho, busca, código e barra de ações.
pub(super) fn edicao_codigo<'a>(app: &'a App) -> Element<'a, Msg> {
    let escuro = app.config.escuro;
    let linha = row![
        botao_campo("(Geral)"),
        Space::with_width(Length::Fill),
        com_dica(botao_icone("Localizar", "search", Some(ev(EventoCodigo::Abrir(false))),
            app.macro_visual.codigo.busca_aberta), "Localizar  Ctrl+F · Substituir  Ctrl+H"),
        botao_campo("(Declarações)"),
    ].spacing(6).align_y(Alignment::Center).padding([7, 10]);
    let achadas = app.ocorrencias_atuais();
    let mut corpo = column![container(linha).style(estilo_cabecalho(escuro))].spacing(3);
    if app.macro_visual.codigo.busca_aberta {
        corpo = corpo.push(barra_busca(app, &achadas));
        if app.macro_visual.codigo.todos_modulos {
            corpo = corpo.push(resultados_todos(app));
        }
    }
    let barra = row![
        text("JAVASCRIPT  ·  MÓDULO").size(10).color(cor_secundaria(escuro)),
        Space::with_width(Length::Fill),
        botao_icone("Guardar  Ctrl+S", "device-floppy", Some(Msg::MacroGuardarModulo), false),
        botao_executar(Some(Msg::MacroExecutar)),
    ].spacing(8).align_y(Alignment::Center).padding([6, 10]);
    container(corpo
        .push(area_codigo(app, &achadas))
        .push(container(barra).style(estilo_cabecalho(escuro)))
        .padding(5))
    .height(Length::Fill).width(Length::Fill)
    .style(estilo_area(escuro)).into()
}

/// Campo "Buscar macro pelo nome" do Explorador (ícone + campo + dica).
pub(super) fn busca_projeto<'a>(app: &'a App) -> Element<'a, Msg> {
    container(row![
        feather_ui::icones::icone("search", 15.0),
        text_input("Buscar macro pelo nome", &app.macro_visual.codigo.filtro_projeto)
            .id(codigo::id_campo_projeto())
            .on_input(|t| ev(EventoCodigo::FiltroProjeto(t)))
            .on_submit(ev(EventoCodigo::FiltroAbrirPrimeiro))
            .size(12).padding([4, 7]).style(feather_ui::tema::estilo_entrada_suspensa),
    ].spacing(6).align_y(Alignment::Center))
        .padding([4, 6]).width(Length::Fill).into()
}

/// Resultado da busca do Explorador: módulos e, abaixo, as funções que bateram.
pub(super) fn resultados_projeto<'a>(app: &'a App) -> Element<'a, Msg> {
    let escuro = app.config.escuro;
    let itens = codigo::filtrar_modulos(&app.macro_catalogo.macros, &app.macro_visual.codigo.filtro_projeto);
    let mut col = column![grupo_arvore(&format!("Resultados ({})", itens.len()), "search", escuro)]
        .spacing(1).padding(4);
    if itens.is_empty() {
        col = col.push(container(column![
            text("Nenhuma macro com esse nome").size(12),
            text("Procura no nome do módulo e das funções, sem diferenciar acento.")
                .size(11).color(COR_TEXTO_MUTED()),
        ].spacing(4)).padding([8, 12]));
    }
    for item in itens {
        let id = format!("modulo:{}", item.modulo);
        col = col.push(item_arvore(
            &item.modulo, if item.auto { "bolt" } else { "file-code" }, 8.0,
            Some(Msg::MacroVisual(EventoVisual::Documento(id.clone()))),
            app.macro_visual.documento == id,
        ));
        for (funcao, linha) in item.funcoes {
            col = col.push(item_arvore(
                &format!("{funcao}  (linha {})", linha + 1), "code", 29.0,
                Some(ev(EventoCodigo::IrParaFuncao(item.modulo.clone(), linha))), false,
            ));
        }
    }
    col.into()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn contador_mostra_posicao_e_total() {
        let achadas = codigo::ocorrencias("a a\na", "a", false, false);
        assert_eq!(contador(&achadas, Some((0, 2)), "a"), "2 de 3");
        assert_eq!(contador(&achadas, None, "a"), "3 resultado(s)");
        assert_eq!(contador(&[], None, "zz"), "Nenhum resultado");
        assert_eq!(contador(&[], None, ""), "");
    }

    #[test]
    fn destaque_so_nas_linhas_e_colunas_inteiras_a_vista() {
        // Sem rolagem conhecida ou texto que cabe: tudo.
        assert_eq!(linhas_visiveis(None, 9_000.0), 0..usize::MAX);
        assert_eq!(linhas_visiveis(Some((0.0, 400.0)), 300.0), 0..usize::MAX);
        // 180 px rolados, 180 px de altura: linhas 10 a 18 (a partir de 0), a 19 cortada.
        let r = linhas_visiveis(Some((180.0, 180.0)), 2_000.0);
        assert_eq!((r.start, r.end), (10, 19));
        assert!(!r.contains(&9) && r.contains(&18) && !r.contains(&19));
        let c = colunas_visiveis(Some((80.0, 160.0)), 2_000.0);
        assert_eq!((c.start, c.end), (9, 28));
    }

    #[test]
    fn calha_cresce_com_os_digitos() {
        assert_eq!(largura_calha(9), largura_calha(99));
        assert!(largura_calha(1000) > largura_calha(99));
    }

    #[test]
    fn editor_monta_com_busca_calha_e_resultados_nos_dois_temas() {
        let mut app = App::default();
        app.macro_catalogo = Default::default();
        app.macro_catalogo.macros.push(feather_macros::Macro {
            name: "Contas".into(), code: "function somar() {}\nsheet.set('A1', 1);".into(), auto: false,
        });
        app.macro_nome = "Macro1".into();
        app.macro_visual.documento = "modulo:Macro1".into();
        app.macro_codigo = text_editor::Content::with_text("const sheet2 = 1;\nlog(sheet2);\n\tsheet.set('B2', 2);");
        for escuro in [false, true] {
            app.config.escuro = escuro;
            let _ = app.evento_codigo_macro(EventoCodigo::Abrir(true));
            let _ = app.evento_codigo_macro(EventoCodigo::Consulta("sheet".into()));
            let _ = app.evento_codigo_macro(EventoCodigo::AlternarTodos);
            let _ = app.evento_codigo_macro(EventoCodigo::FiltroProjeto("SOMAR".into()));
            let _ = janela(&app);
            let _ = app.evento_codigo_macro(EventoCodigo::FiltroProjeto("nada".into()));
            let _ = app.evento_codigo_macro(EventoCodigo::Fechar);
            let _ = janela(&app);
        }
    }
}
