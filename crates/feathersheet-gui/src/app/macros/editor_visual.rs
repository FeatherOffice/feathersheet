//! Espelho visual inicial do editor VBE TypeScript (pagina.ts/css.ts).
//! A GUI abaixo é feita somente de componentes Rust/Iced. Comandos que ainda
//! dependem do TS são exibidos sem on_press, para não prometer execução.
//! O motor de macros permanece no featheroffice-macros compartilhado.

use super::*;
use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

pub(in crate::app) mod estilos;
mod console;
mod barra_menu;
mod historico;
mod codigo_view;
use estilos::*;
use console::{rodape, terminal};
use barra_menu::{ferramentas, menus};
#[cfg(test)]
use barra_menu::menu_itens;

/// Encaminhamento puro: reaproveita o Msg real do menu legado sem criar
/// comandos de fachada para entradas ainda indisponíveis.
pub(in crate::app) fn acao_menu_visual(id: u8, indice: usize) -> Option<Msg> {
    barra_menu::menu_itens(id).into_iter().nth(indice).and_then(|(_, msg)| msg)
}

/// Painel do console inferior que mostra o histórico de macros.
pub(in crate::app) const PAINEL_HISTORICO: u8 = 4;

const MENUS: [&str; 8] = [
    "Arquivo", "Editar", "Exibir", "Inserir",
    "Executar", "Depurar", "Ferramentas", "Ajuda",
];

fn mensagem_visual<'a>(nome: &str) -> Element<'a, Msg> {
    container(column![
        text(nome.to_owned()).size(14),
        text(tr!("Estrutura visual migrada do editor TypeScript.")).size(12),
        text(tr!("Os comandos dessa área serão conectados ao motor comum em outra etapa."))
            .size(11),
    ].spacing(8))
        .padding(20).width(Length::Fill).height(Length::Fill)
        .center_x(Length::Fill).center_y(Length::Fill)
        .into()
}
fn event(e: EventoVisual) -> Option<Msg> { Some(Msg::MacroVisual(e)) }

fn arvore<'a>(app: &'a App) -> Element<'a, Msg> {
    let escuro = app.config.escuro;
    let mut itens = column![
        item_arvore(
            tr!("FeatherSheet (Pasta de trabalho)"), "folder-open", 0.0,
            event(EventoVisual::Documento("pasta".into())),
            app.macro_visual.documento == "pasta",
        ),
        grupo_arvore(tr!("Planilhas"), "table", escuro),
    ].spacing(1).padding(4);
    for nome in app.workbook.nomes().iter().take(48) {
        let id = format!("planilha:{nome}");
        itens = itens.push(item_arvore(
            nome, "table", 21.0,
            event(EventoVisual::Documento(id.clone())),
            app.macro_visual.documento == id,
        ));
    }
    itens = itens.push(item_arvore(
        tr!("Esta pasta de trabalho"), "window", 21.0,
        event(EventoVisual::Documento("pasta".into())),
        app.macro_visual.documento == "pasta",
    ));
    itens = itens.push(grupo_arvore(tr!("Módulos"), "file-code", escuro));
    let modulo_atual = format!("modulo:{}", app.macro_nome);
    if app.macro_catalogo.macros.is_empty() {
        itens = itens.push(item_arvore(
            &trf!("{} (rascunho)", app.macro_nome), "file-code", 21.0,
            event(EventoVisual::Documento(modulo_atual.clone())),
            app.macro_visual.documento == modulo_atual,
        ));
    } else {
        for modulo in app.macro_catalogo.macros.iter().take(60) {
            let id = format!("modulo:{}", modulo.name);
            let rotulo = format!("{}{}", modulo.name,
                if modulo.auto { tr!(" (automática)") } else { "" });
            itens = itens.push(item_arvore(
                &rotulo, if modulo.auto { "bolt" } else { "file-code" }, 21.0,
                event(EventoVisual::Documento(id.clone())),
                app.macro_visual.documento == id,
            ));
        }
        if app.macro_catalogo.obter(&app.macro_nome).is_none() {
            itens = itens.push(item_arvore(
                &trf!("{} (rascunho)", app.macro_nome), "file-code", 21.0,
                event(EventoVisual::Documento(modulo_atual.clone())),
                app.macro_visual.documento == modulo_atual,
            ));
        }
    }
    itens = itens.push(grupo_arvore(tr!("Componentes"), "forms", escuro));
    itens = itens.push(item_arvore(
        tr!("Componente1 (prévia)"), "forms", 21.0,
        event(EventoVisual::Documento("componente:previsao".into())),
        app.macro_visual.documento == "componente:previsao",
    ));
    let cabecalho = row![
        text(tr!("Explorador de projeto")).size(13),
        Space::with_width(Length::Fill),
        botao_icone_simples("search", Some(Msg::MacroVisual(EventoVisual::Codigo(
            EventoCodigo::FocarProjeto)))),
        botao_icone_simples("plus", Some(Msg::MacroNovoModulo)),
    ].spacing(2).align_y(Alignment::Center);
    // Com a busca preenchida, a árvore dá lugar aos módulos/funções achados.
    let lista: Element<'a, Msg> = if app.macro_visual.codigo.filtro_projeto.trim().is_empty() {
        itens.into()
    } else {
        codigo_view::resultados_projeto(app)
    };
    container(column![
        container(cabecalho).padding([5, 9]).style(estilo_cabecalho(escuro)),
        codigo_view::busca_projeto(app),
        scrollable(lista).height(Length::Fill),
    ].spacing(3).padding(4))
        .style(estilo_area(escuro)).height(Length::FillPortion(3))
        .width(Length::Fill).into()
}

fn propriedades<'a>(app: &'a App) -> Element<'a, Msg> {
    let escuro = app.config.escuro;
    let v = &app.macro_visual;
    let tipo = if v.documento.starts_with("modulo:") { tr!("Módulo JavaScript") }
        else if v.documento.starts_with("planilha:") { tr!("Planilha") }
        else if v.documento.starts_with("componente:") { tr!("Formulário") }
        else { tr!("Pasta de trabalho") };
    let nome = v.documento.split_once(':').map_or("Esta pasta de trabalho", |(_, n)| n);
    let linhas = app.macro_codigo.text().lines().count();
    let tem_auto = app.macro_catalogo.obter(nome).is_some_and(|m| m.auto);
    let mut pares = column![].spacing(1).padding(5);
    if v.propriedades_categoria {
        pares = pares.push(grupo_arvore(tr!("Geral"), "list-details", escuro));
    }
    let dados = [
        ("(Nome)", nome.to_owned()),
        (tr!("Tipo"), tipo.to_owned()),
        (tr!("Executar ao abrir"), if tem_auto { tr!("Sim (TS)") } else { tr!("Não") }.to_owned()),
        (tr!("Linhas"), if tipo == "Módulo JavaScript" { linhas.to_string() } else { "—".into() }),
        (tr!("Linguagem"), if tipo == "Módulo JavaScript" { "JavaScript" } else { "—" }.to_owned()),
    ];
    for (chave, valor) in dados {
        pares = pares.push(row![
            container(text(chave).size(11)).width(Length::FillPortion(5)).padding([7, 8])
                .style(estilo_celula_propriedade(escuro, true)),
            container(text(valor).size(11)).width(Length::FillPortion(6)).padding([7, 8])
                .style(estilo_celula_propriedade(escuro, false)),
        ].spacing(0));
    }
    let barras = row![
        botao_icone(tr!("Alfabética"), "sort-ascending", event(EventoVisual::PropriedadesCategoria(false)),
            !v.propriedades_categoria),
        botao_icone(tr!("Por categoria"), "list-details", event(EventoVisual::PropriedadesCategoria(true)),
            v.propriedades_categoria),
    ].spacing(4).padding(5);
    container(column![
        container(row![
            text(tr!("Propriedades")).size(13),
            Space::with_width(Length::Fill),
            feather_ui::icones::icone("list-details", 15.0),
        ].align_y(Alignment::Center)).padding([11, 12])
            .style(estilo_cabecalho(escuro)),
        barras,
        scrollable(pares).height(Length::Fill),
        row![
            botao_icone(tr!("Renomear"), "edit", None, false),
            botao_icone(tr!("Excluir"), "trash", if v.documento.starts_with("modulo:") {
                Some(Msg::MacroExcluirModulo)
            } else { None }, false),
        ].spacing(5).padding([6, 7]),
    ].spacing(3).padding(4))
    .style(estilo_area(escuro)).width(Length::Fill)
    .height(Length::FillPortion(2)).into()
}

fn painel_lateral<'a>(app: &'a App) -> Element<'a, Msg> {
    let mut conteudo = column![].spacing(8).height(Length::Fill);
    if app.macro_visual.explorador { conteudo = conteudo.push(arvore(app)); }
    if app.macro_visual.propriedades { conteudo = conteudo.push(propriedades(app)); }
    container(conteudo).width(Length::Fixed(280.0)).height(Length::Fill).into()
}

fn abas_documento<'a>(app: &'a App) -> Element<'a, Msg> {
    let escuro = app.config.escuro;
    let atual = &app.macro_visual.documento;
    let mut abas = row![].spacing(3).width(Length::Shrink);
    let (nome_atual, icone_atual) = if atual == "pasta" {
        (tr!("Esta pasta de trabalho").to_string(), "window")
    } else if let Some(nome) = atual.strip_prefix("planilha:") {
        (nome.to_owned(), "table")
    } else if atual.starts_with("componente:") {
        ("Componente1".to_string(), "forms")
    } else {
        (atual.strip_prefix("modulo:").unwrap_or(&app.macro_nome).to_owned(), "file-code")
    };
    abas = abas.push(botao_aba(
        &nome_atual, icone_atual,
        event(EventoVisual::Documento(atual.clone())), true,
    ));
    if !atual.starts_with("modulo:") {
        abas = abas.push(botao_aba(
            &app.macro_nome, "file-code",
            event(EventoVisual::Documento(format!("modulo:{}", app.macro_nome))), false,
        ));
    }
    if atual != "componente:previsao" {
        abas = abas.push(botao_aba(
            tr!("Formulário (prévia)"), "forms",
            event(EventoVisual::Documento("componente:previsao".into())), false,
        ));
    }
    abas = abas.push(botao_icone_simples("plus", Some(Msg::MacroNovoModulo)));
    container(scrollable(abas).direction(scrollable::Direction::Horizontal(
        scrollable::Scrollbar::new(),
    )).height(Length::Fixed(43.0)))
        .width(Length::Fill).padding([4, 6]).style(estilo_cabecalho(escuro)).into()
}

fn edicao_codigo<'a>(app: &'a App) -> Element<'a, Msg> {
    // Busca, calha de números e rolagem própria: `codigo_view.rs`.
    codigo_view::edicao_codigo(app)
}

fn evento_planilha<'a>(app: &'a App) -> Element<'a, Msg> {
    let codigo = app.macro_visual.documento.starts_with("planilha:");
    let seletor = if codigo { "SelectionChange" }
        else { "Open / BeforeClose" };
    container(column![
        titulo(tr!("CÓDIGO DE EVENTOS  ·  ESTRUTURA TS"), app.config.escuro),
        row![
            botao_campo("(Geral)"),
            botao_campo(seletor),
            Space::with_width(Length::Fill),
            botao_icone(tr!("Eventos"), "bolt", None, false),
        ].padding(7),
        mensagem_visual(tr!("Eventos da planilha e da pasta")),
    ]).width(Length::Fill).height(Length::Fill)
        .style(estilo_area(app.config.escuro)).into()
}

fn aba_componente<'a>(app: &'a App) -> Element<'a, Msg> {
    let escuro = app.config.escuro;
    let active = app.macro_visual.componente_aba;
    let tabs = row![
        botao_icone(tr!("Desenho"), "layout-grid", event(EventoVisual::ComponenteAba(0)), active == 0),
        botao_icone("HTML", "code", event(EventoVisual::ComponenteAba(1)), active == 1),
        botao_icone("CSS", "palette", event(EventoVisual::ComponenteAba(2)), active == 2),
        Space::with_width(Length::Fill),
        botao_icone(tr!("Prévia"), "eye", None, false),
        botao_icone(tr!("Soltar do desenho"), "arrows-maximize", None, false),
    ].spacing(4).padding([4, 7]);
    let corpo: Element<'a, Msg> = if active == 0 {
        let toolbox = column![
            titulo(tr!("CAIXA DE FERRAMENTAS"), escuro),
            text(tr!("Controles do formulário")).size(11).color(cor_secundaria(escuro)),
            botao_icone(tr!("Selecionar"), "pointer", None, false),
            botao_icone(tr!("Rótulo"), "text-size", None, false),
            botao_icone(tr!("Campo de texto"), "input", None, false),
            botao_icone(tr!("Campo numérico"), "sum", None, false),
            botao_icone(tr!("Senha"), "lock", None, false),
            botao_icone(tr!("Caixa de seleção"), "checkbox", None, false),
            botao_icone(tr!("Opção"), "circle-dot", None, false),
            botao_icone(tr!("Lista suspensa"), "list-select", None, false),
            botao_icone(tr!("Botão"), "square", None, false),
        ].spacing(5).padding(8);
        let formulario = container(column![
            titulo(tr!("Formulário1  ·  desenho"), escuro),
            text(tr!("Título do formulário")).size(17),
            text(tr!("Nome")).size(12).color(cor_secundaria(escuro)),
            botao(tr!("Campo de texto"), None, false),
            row![
                botao_icone(tr!("Confirmar"), "circle-check", None, true),
                botao_icone(tr!("Cancelar"), "x", None, false),
            ].spacing(6),
            Space::with_height(Length::Fill),
            text(tr!("Prévia visual; arrastar e configurar controles virá depois."))
                .size(11).color(cor_secundaria(escuro)),
        ].spacing(10).padding(24))
        .width(Length::Fill).height(Length::Fill)
        .style(estilo_area(escuro));
        row![
            container(toolbox).width(Length::Fixed(182.0)).height(Length::Fill)
                .style(estilo_cabecalho(escuro)),
            container(formulario).padding(18).width(Length::Fill).height(Length::Fill),
        ].height(Length::Fill).into()
    } else {
        let linguagem = if active == 1 { tr!("HTML — editor do componente") } else { tr!("CSS — estilo do componente") };
        row![
            mensagem_visual(linguagem),
            container(column![
                titulo(tr!("PRÉVIA DO FORMULÁRIO"), escuro),
                text(tr!("Formulário1")).size(14),
                text(tr!("Nome")).size(11),
                botao(tr!("Campo de texto"), None, false),
                botao(tr!("Confirmar"), None, true),
            ].spacing(8).padding(12))
              .width(Length::Fixed(228.0)).height(Length::Fill).style(estilo_area(escuro)),
        ].height(Length::Fill).into()
    };
    column![
        container(tabs).style(estilo_cabecalho(escuro)),
        corpo,
    ].height(Length::Fill).into()
}

fn centro<'a>(app: &'a App) -> Element<'a, Msg> {
    let c: Element<'a, Msg> = if app.macro_visual.documento == "componente:previsao" {
        aba_componente(app)
    } else if app.macro_visual.documento.starts_with("modulo:") {
        edicao_codigo(app)
    } else {
        evento_planilha(app)
    };
    column![abas_documento(app), c].spacing(5)
        .width(Length::Fill).height(Length::Fill).into()
}

/// Montagem central do VBE: menus → toolbar → explorador/propriedades + editor
/// → janelas de depuração → barra de status. Nenhum HTML ou iframe é usado.
pub(super) fn janela<'a>(app: &'a App) -> Element<'a, Msg> {
    let mut corpo = row![].spacing(8).height(Length::Fill);
    if app.macro_visual.explorador || app.macro_visual.propriedades {
        corpo = corpo.push(painel_lateral(app));
    }
    corpo = corpo.push(centro(app));
    let mut estrutura = column![
        menus(app),
        ferramentas(app),
        corpo,
    ].spacing(7).height(Length::Fill).width(Length::Fill);
    if app.macro_visual.console {
        estrutura = estrutura.push(terminal(app));
    }
    estrutura = estrutura.push(rodape(app));
    let base = container(estrutura).padding(8)
        .style(estilo_fundo_editor(app.config.escuro))
        .height(Length::Fill).width(Length::Fill);
    if let Some(id) = app.macro_visual.menu {
        // Flutua sobre a toolbar/editor. Não insere nova linha no layout:
        // eliminamos o espaço vazio que aparecia abaixo do menu.
        let popup = column![
            Space::with_height(Length::Fixed(44.0)),
            row![
                Space::with_width(Length::Fixed(barra_menu::posicao_popup(id))),
                iced::widget::opaque(barra_menu::menu_suspenso(app, id)),
            ].spacing(0),
        ].spacing(0);
        // Clicar fora do dropdown o fecha; o cabeçalho permanece ativo
        // para alternar diretamente entre Arquivo/Editar/Exibir/etc.
        let fechar_fora = column![
            Space::with_height(Length::Fixed(44.0)),
            iced::widget::mouse_area(
                Space::new(Length::Fill, Length::Fill)
            ).on_press(Msg::MacroVisual(EventoVisual::FecharMenu)),
        ].spacing(0).height(Length::Fill).width(Length::Fill);
        iced::widget::stack![base, fechar_fora, popup]
            .height(Length::Fill).width(Length::Fill).into()
    } else {
        base.into()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn janela_monta_todas_as_areas_nas_duas_aparencias() {
        let mut app = App::default();
        for escuro in [false, true] {
            app.config.escuro = escuro;
            for aba in ["modulo:Macro1", "pasta", "componente:previsao"] {
                app.evento_macro_visual(EventoVisual::Documento(aba.to_string()));
                for painel in 0..=PAINEL_HISTORICO {
                    app.evento_macro_visual(EventoVisual::Painel(painel));
                    let _ = janela(&app);
                }
            }
            app.evento_macro_visual(EventoVisual::Explorador);
            app.evento_macro_visual(EventoVisual::Propriedades);
            app.evento_macro_visual(EventoVisual::Console);
            let _ = janela(&app);
        }
    }

    #[test]
    fn aparencia_clean_monta_abas_toolbar_e_console_em_claro_escuro() {
        let mut app = App::default();
        for escuro in [false, true] {
            app.config.escuro = escuro;
            app.macro_visual.documento = format!("modulo:{}", app.macro_nome);
            let _: Element<'_, Msg> = menus(&app);
            let _: Element<'_, Msg> = ferramentas(&app);
            let _: Element<'_, Msg> = abas_documento(&app);
            let _: Element<'_, Msg> = terminal(&app);
            let _: Element<'_, Msg> = rodape(&app);
            let _: Element<'_, Msg> = janela(&app);
        }
    }

    #[test]
    fn editor_horizontal_sem_panic_ao_abrir_menus_e_documentos() {
        // Regressão do panic iced_widget::scrollable.rs:
        // "scrollable content must not fill its horizontal scrolling axis".
        // O layout é construído como na GUI: os menus agora são dropdowns
        // sobrepostos; nunca criam faixa horizontal adicional nem scroll.
        let mut app = App::default();
        let _ = ferramentas(&app);
        let _ = abas_documento(&app);
        for menu in 0..MENUS.len() {
            app.macro_visual.menu = Some(menu as u8);
            let _ = menus(&app);
            let _ = janela(&app);
        }
        for documento in [
            "pasta".to_owned(), "componente:previsao".to_owned(),
            format!("modulo:{}", app.macro_nome),
        ] {
            app.macro_visual.documento = documento;
            let _ = abas_documento(&app);
            let _ = janela(&app);
        }
        // A barra de execução nunca deve levar Fill como conteúdo do
        // scroll horizontal, mesmo com painéis esquerdo/inferior abertos.
        app.macro_visual.explorador = true;
        app.macro_visual.propriedades = true;
        app.macro_visual.console = true;
        let _ = janela(&app);
    }

    #[test]
    fn todos_os_icones_macros_estao_no_catalogo_compartilhado() {
        // Se uma chave não estiver cadastrada no feather-ui, a função de
        // desenho usaria o ícone genérico (+), ocultando o problema.
        let usados = [
            "folder-open", "folder", "table", "window", "file-code",
            "chevron-down", "forms", "bolt", "sitemap", "terminal",
            "list-details", "sort-ascending", "edit", "trash", "code",
            "palette", "eye", "arrows-maximize", "pointer", "text-size",
            "input", "sum", "lock", "checkbox", "circle-dot", "list-select",
            "square", "circle-check", "player-play", "player-pause",
            "player-stop", "arrow-down", "arrow-step", "arrow-back-up",
            "help", "device-floppy", "plus", "refresh", "x", "search",
            "file-plus", "file-export", "checks", "settings", "keyboard",
            "list", "info-circle", "clock", "bug", "message-circle",
            "chevron-up", "chevron-right", "file-code",
        ];
        for chave in usados {
            assert!(feather_ui::icones::existe(chave),
                "Ícone SVG ausente no catálogo: {chave}");
        }
        for menu in 0..MENUS.len() {
            for (rotulo, _) in menu_itens(menu as u8) {
                let nome = barra_menu::icone_do_menu(rotulo)
                    .unwrap_or_else(|| panic!("Menu sem ícone: {rotulo}"));
                assert!(feather_ui::icones::existe(nome),
                    "Ícone {nome} não encontrado para {rotulo}");
            }
        }
    }

    #[test]
    fn menus_suspensos_sao_individuais_e_nao_aumentam_altura_da_barra() {
        let mut app = App::default();
        let _ = menus(&app);
        for indice in 0..MENUS.len() {
            app.macro_visual.menu = Some(indice as u8);
            // Deve montar o mesmo corpo da janela, com popup sobreposto.
            let _: Element<'_, Msg> = barra_menu::menu_suspenso(&app, indice as u8);
            let _: Element<'_, Msg> = janela(&app);
        }
        assert!(barra_menu::posicao_popup(1) > barra_menu::posicao_popup(0));
        assert!(barra_menu::posicao_popup(7) < 600.0);
    }

    #[test]
    fn estrutura_visual_reune_todos_os_menus_ts() {
        assert_eq!(MENUS, [
            "Arquivo", "Editar", "Exibir", "Inserir",
            "Executar", "Depurar", "Ferramentas", "Ajuda",
        ]);
        for menu in 0..8 {
            assert!(!menu_itens(menu).is_empty());
        }
        // Comandos do depurador e novas rotinas aparecem visualmente,
        // mas sem Mensagem de execução enquanto não forem implementados.
        assert!(menu_itens(5).iter().any(|(r,m)| r.contains("Pausar") && m.is_none()));
        assert!(menu_itens(0).iter().any(|(r,m)| r.contains("Salvar .js") && m.is_some()));
    }
}
