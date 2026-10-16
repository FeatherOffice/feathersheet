//! Menus suspensos da Página Inicial: AutoSoma, Preencher, Classificar/Filtrar,
//! Formatar (grupo Células) e Mesclar.
//! Painéis ancorados à Ribbon, sem substituir o conteúdo da planilha.
use super::*;

const LARGURA: f32 = 262.0;

/// Linha de um menu suspenso: rótulo de seção, comando ou divisória.
enum Item {
    Titulo(&'static str),
    Acao(&'static str, &'static str, Msg),
    Divisor,
}

fn itens(tipo: &str) -> Vec<Item> {
    use Item::{Acao, Divisor, Titulo};
    match tipo {
        "autosoma" => vec![
            Titulo("INSERIR FUNÇÃO"),
            Acao("Soma", "sum", Msg::AutoSoma),
            Acao("Média", "math-function", Msg::InserirFuncao("MÉDIA".into())),
            Acao("Contagem", "list-numbers", Msg::InserirFuncao("COUNT".into())),
            Acao("Máximo", "sort-descending", Msg::InserirFuncao("MAX".into())),
            Acao("Mínimo", "sort-ascending", Msg::InserirFuncao("MIN".into())),
        ],
        "preencher" => vec![
            Titulo("PREENCHER CÉLULAS"),
            Acao("Para baixo · Ctrl+D", "arrow-down", Msg::Preencher(DirecaoPreenchimento::Baixo)),
            Acao("Para direita · Ctrl+R", "seta_direita", Msg::Preencher(DirecaoPreenchimento::Direita)),
            Acao("Para cima", "sort-ascending", Msg::Preencher(DirecaoPreenchimento::Cima)),
            Acao("Para esquerda", "seta_esquerda", Msg::Preencher(DirecaoPreenchimento::Esquerda)),
            Acao("Continuar até o fim", "arrow-step", Msg::Preencher(DirecaoPreenchimento::AteOFim)),
            Acao("Série…", "list-numbers", Msg::Serie(serie_ui::Evento::Abrir)),
            Divisor,
            Acao("Preenchimento relâmpago · Ctrl+E", "bolt", Msg::PreenchimentoRelampago),
        ],
        // Validação ▾ da aba Dados, como no Excel.
        "validacao" => vec![
            Titulo("VALIDAÇÃO DE DADOS"),
            Acao("Validação de dados…", "checks", Msg::ValidacaoAbrir),
            Acao("Circular dados inválidos", "circle", Msg::ValidacaoEv(validacao_dialogo::Evento::Circular)),
            Acao("Limpar círculos de validação", "eraser",
                Msg::ValidacaoEv(validacao_dialogo::Evento::LimparCirculos)),
        ],
        // Formatar ▾ do grupo Células, na ordem do Excel.
        "formatar" => vec![
            Titulo("TAMANHO DA CÉLULA"),
            Acao("Altura da linha…", "line-height",
                Msg::Dimensao(dimensao_dialogo::Evento::Abrir(false))),
            Acao("AutoAjuste da altura da linha", "arrow-autofit-height", Msg::AjustarAltura),
            Acao("Largura da coluna…", "arrow-autofit-width",
                Msg::Dimensao(dimensao_dialogo::Evento::Abrir(true))),
            Acao("AutoAjuste da largura da coluna", "arrows-horizontal", Msg::AjustarLargura),
            Acao("Largura padrão…", "arrow-autofit-width", Msg::Dimensao(dimensao_dialogo::Evento::AbrirPadrao)),
            Titulo("VISIBILIDADE"),
            Acao("Ocultar linhas", "eye-off", Msg::OcultarDimensoes(false)),
            Acao("Ocultar colunas", "eye-off", Msg::OcultarDimensoes(true)),
            Acao("Reexibir linhas", "eye", Msg::ReexibirDimensoes(false)),
            Acao("Reexibir colunas", "eye", Msg::ReexibirDimensoes(true)),
            Divisor,
            Acao("Formatar células…", "edit", Msg::FormatarCelulasAbrir),
            Divisor,
            Titulo("PROTEÇÃO"),
            Acao("Proteger ou desproteger planilha…", "lock", Msg::Protecao(protecao_ui::Evento::Abrir)),
            Acao("Bloquear/desbloquear célula", "lock", Msg::Protecao(protecao_ui::Evento::AlternarBloqueio)),
        ],
        // Inserir ▾ / Excluir ▾ do grupo Células, como no Excel.
        "inserir" => vec![
            Titulo("INSERIR"),
            Acao("Inserir células… · Ctrl++", "table", Msg::Celulas(celulas_dialogo::Evento::Abrir(true))),
            Acao("Inserir linhas na planilha", "plus", Msg::InserirMenu(Estrutura::InserirLinha, false)),
            Acao("Inserir colunas na planilha", "columns", Msg::InserirMenu(Estrutura::InserirColuna, false)),
            Acao("Inserir planilha · Shift+F11", "file-plus", Msg::AdicionarAba),
        ],
        "excluir" => vec![
            Titulo("EXCLUIR"),
            Acao("Excluir células… · Ctrl+-", "table", Msg::Celulas(celulas_dialogo::Evento::Abrir(false))),
            Acao("Excluir linhas da planilha", "trash", Msg::Estrutura(Estrutura::ExcluirLinha)),
            Acao("Excluir colunas da planilha", "columns", Msg::Estrutura(Estrutura::ExcluirColuna)),
        ],
        // Limpar ▾ do grupo Edição, como no Excel.
        "limpar" => vec![
            Titulo("LIMPAR"),
            Acao("Limpar tudo", "eraser", Msg::LimparTudo),
            Acao("Limpar formatos", "eraser", Msg::LimparFormato),
            Acao("Limpar conteúdo · Delete", "eraser", Msg::Limpar),
            Acao("Limpar comentários", "message-circle", Msg::Comentario(comentarios_ui::Evento::Limpar)),
            Acao("Limpar hiperlinks", "link-off", Msg::LimparHiperlinks),
        ],
        // Sublinhado ▾ do grupo Fonte, como no Excel.
        "sublinhado" => vec![
            Titulo("SUBLINHADO"),
            Acao("Sublinhado · Ctrl+U", "underline", Msg::AlternarEstilo("underline".into())),
            Acao("Sublinhado duplo", "underline-double",
                Msg::AlternarEstilo(feathersheet_core::operacoes::sublinhado::CHAVE_DUPLO.into())),
        ],
        "mesclar" => vec![
            Titulo("MESCLAR"),
            Acao("Mesclar e centralizar", "align-center",
                Msg::MesclarModo(mesclas::ModoMescla::Centralizar)),
            Acao("Mesclar através", "columns", Msg::MesclarModo(mesclas::ModoMescla::Atraves)),
            Acao("Mesclar células", "square", Msg::MesclarModo(mesclas::ModoMescla::Mesclar)),
            Acao("Desmesclar células", "desmesclar_celulas",
                Msg::MesclarModo(mesclas::ModoMescla::Desmesclar)),
        ],
        _ => vec![
            Titulo("CLASSIFICAR E FILTRAR"),
            Acao("Classificar de A a Z", "sort-ascending", Msg::Ordenar(true)),
            Acao("Classificar de Z a A", "sort-descending", Msg::Ordenar(false)),
            Acao("Classificação personalizada…", "sort-ascending",
                Msg::Classificar(classificar::Evento::Abrir)),
            Acao("Ativar/desativar filtro", "filter",
                Msg::TabelaAcao(tabelas::Acao::AlternarAutoFiltro)),
            Acao("Reaplicar", "refresh", Msg::TabelaAcao(tabelas::Acao::Reaplicar)),
            Acao("Selecionar região atual", "table", Msg::SelecionarRegiaoAtual),
        ],
    }
}

/// Altura estimada do painel, para não deixá-lo passar da borda da janela.
fn altura(tipo: &str) -> f32 {
    20.0 + itens(tipo).iter().map(|item| match item {
        Item::Titulo(_) => 22.0,
        Item::Acao(..) => 37.0,
        Item::Divisor => 9.0,
    }).sum::<f32>()
}

impl App {
    /// O menu fica junto ao botão acionado, sem ocupar a área de trabalho.
    fn menu_edicao(&self, tipo: &'static str) -> Element<'_, Msg> {
        let mut coluna = iced::widget::column![].spacing(3);
        for (i, item) in itens(tipo).into_iter().enumerate() {
            coluna = match item {
                Item::Titulo(titulo) => coluna.push(
                    container(text(titulo).size(11).color(COR_TEXTO_MUTED()))
                        .padding(iced::Padding { top: if i == 0 { 0.0 } else { 6.0 }, ..Default::default() }),
                ),
                Item::Acao(rotulo, icone, msg) => coluna.push(
                    button(row![
                        feather_ui::icones::icone(icone, 17.),
                        text(rotulo).size(12),
                    ].spacing(9).align_y(Alignment::Center))
                        .width(Length::Fill)
                        .padding([8, 10])
                        .style(estilo_botao_pequeno(false))
                        .on_press(Msg::EdicaoMenuAcao(Box::new(msg))),
                ),
                Item::Divisor => coluna.push(
                    container(iced::widget::horizontal_rule(1)).padding([4, 0]),
                ),
            };
        }
        container(coluna).padding(10).width(Length::Fixed(LARGURA))
            .style(estilo_popup_menu).into()
    }

    pub(super) fn sobrepor_menu_edicao<'a>(
        &'a self, base: Element<'a, Msg>,
    ) -> Element<'a, Msg> {
        let Some(tipo) = self.edicao_menu else { return base; };
        let altura = altura(tipo);
        let x = self.edicao_popup_x.clamp(
            8.0, (self.tamanho_principal.width - LARGURA - 8.0).max(8.0),
        );
        let y = self.edicao_popup_y.clamp(
            8.0, (self.tamanho_principal.height - altura - 8.0).max(8.0),
        );
        let fora = iced::widget::mouse_area(
            Space::new(Length::Fill, Length::Fill)
        ).on_press(Msg::FecharMenuEdicao);
        stack![
            base, fora,
            iced::widget::column![
                Space::with_height(Length::Fixed(y)),
                row![
                    Space::with_width(Length::Fixed(x)),
                    self.menu_edicao(tipo),
                ],
            ],
        ].height(Length::Fill).into()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn menus_flutuantes_de_edicao_preservam_grade() {
        let mut app = App::default();
        app.update(Msg::AlternarMenuEdicao("autosoma"));
        assert_eq!(app.edicao_menu, Some("autosoma"));
        let _: Element<'_, Msg> = app.menu_edicao("autosoma");
        let _: Element<'_, Msg> = app.menu_edicao("preencher");
        let _: Element<'_, Msg> = app.menu_edicao("formatar");
        let _: Element<'_, Msg> = app.menu_edicao("mesclar");
        app.update(Msg::AlternarMenuEdicao("classificar"));
        assert_eq!(app.edicao_menu, Some("classificar"));
        app.update(Msg::FecharMenuEdicao);
        assert!(app.edicao_menu.is_none());
    }

    #[test]
    fn icones_dos_menus_existem_no_catalogo() {
        for tipo in ["autosoma", "preencher", "classificar", "formatar", "mesclar", "validacao", "limpar", "sublinhado"] {
            for item in itens(tipo) {
                if let Item::Acao(rotulo, icone, _) = item {
                    assert!(feather_ui::icones::existe(icone), "{tipo}: {rotulo} ({icone})");
                }
            }
        }
    }

    #[test]
    fn formatar_abre_o_dialogo_de_altura_pelo_menu() {
        let mut app = App::default();
        app.update(Msg::AlternarMenuEdicao("formatar"));
        app.update(Msg::EdicaoMenuAcao(Box::new(
            Msg::Dimensao(dimensao_dialogo::Evento::Abrir(false)))));
        assert!(app.edicao_menu.is_none());
        assert!(app.dimensao_modal.as_ref().is_some_and(|m| !m.coluna));
    }
}

#[cfg(test)]
mod testes_recuo_limpar {
    use super::*;
    use feathersheet_core::operacoes::recuo::recuo_de;

    #[test]
    fn botoes_de_recuo_e_limpar_tudo() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 0, 0, "abc".into());
        let _ = app.update(Msg::AlterarRecuo(1));
        let _ = app.update(Msg::AlterarRecuo(1));
        assert_eq!(recuo_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), 2);
        let _ = app.update(Msg::AlterarRecuo(-1));
        assert_eq!(recuo_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), 1);
        // Limpar formatos tira o recuo e deixa o texto.
        let _ = app.update(Msg::LimparFormato);
        assert_eq!(recuo_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), 0);
        assert_eq!(app.workbook.aba(0).unwrap().sheet.get_raw(0, 0), "abc");
        // Limpar tudo tira conteúdo e formato numa só ação desfazível.
        let _ = app.update(Msg::AlterarRecuo(3));
        let _ = app.update(Msg::LimparTudo);
        let sheet = &app.workbook.aba(0).unwrap().sheet;
        assert_eq!(sheet.get_raw(0, 0), "");
        assert_eq!(recuo_de(sheet, 0, 0), 0);
        let _ = app.update(Msg::Desfazer);
        let sheet = &app.workbook.aba(0).unwrap().sheet;
        assert_eq!(sheet.get_raw(0, 0), "abc");
        assert_eq!(recuo_de(sheet, 0, 0), 3);
    }

    #[test]
    fn menu_limpar_existe_com_cinco_acoes() {
        // Limpar tudo, formatos, conteúdo, comentários e hiperlinks, como no Excel.
        assert_eq!(itens("limpar").iter().filter(|i| matches!(i, Item::Acao(..))).count(), 5);
    }

    #[test]
    fn limpar_hiperlinks_tira_o_link_da_selecao_e_mantem_o_texto() {
        let mut app = App::default();
        for r in 0..3 {
            app.workbook.escrever_celula(0, r, 0, format!("site {r}"));
        }
        let mut patch = EstiloParcial::new();
        patch.insert("link".into(), Campo::Texto("https://exemplo.com.br".into()));
        patch.insert("bold".into(), Campo::Logico(true));
        app.workbook.editar(0, |s| s.set_style(&Rect::new(0, 0, 2, 0), &patch));
        let _ = app.update(Msg::Selecionar(0, 0));
        let _ = app.update(Msg::Estender(1, 0));
        let _ = app.update(Msg::EdicaoMenuAcao(Box::new(Msg::LimparHiperlinks)));
        let link = |app: &App, r| app.workbook.aba(0).unwrap().sheet.estilo_resolvido(r, 0).unwrap().link;
        assert!(link(&app, 0).is_none() && link(&app, 1).is_none());
        assert!(link(&app, 2).is_some(), "fora da seleção não muda");
        let s = &app.workbook.aba(0).unwrap().sheet;
        assert_eq!(s.get_raw(1, 0), "site 1");
        assert!(s.estilo_resolvido(1, 0).unwrap().bold, "formatação preservada");
        let _ = app.update(Msg::Desfazer);
        assert!(link(&app, 0).is_some() && link(&app, 1).is_some(), "um Ctrl+Z só");
    }

    #[test]
    fn sublinhado_duplo_pelo_menu_e_recuo_pelo_teclado() {
        use feathersheet_core::operacoes::sublinhado::{sublinhado_de, Sublinhado};
        let mut app = App::default();
        let id = window::Id::unique();
        app.janelas.principal = Some(id);
        app.workbook.escrever_celula(0, 0, 0, "abc".into());
        let _ = app.update(Msg::AlternarMenuEdicao("sublinhado"));
        let _: Element<'_, Msg> = app.menu_edicao("sublinhado");
        let Item::Acao(_, _, duplo) = itens("sublinhado").remove(2) else { panic!() };
        let _ = app.update(Msg::EdicaoMenuAcao(Box::new(duplo)));
        assert!(app.edicao_menu.is_none());
        assert_eq!(sublinhado_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), Sublinhado::Duplo);
        // Ctrl+U troca o duplo pelo simples; de novo, tira.
        let ctrl = keyboard::Modifiers::CTRL;
        let _ = app.update(Msg::Teclado(id, keyboard::Key::Character("u".into()), ctrl));
        assert_eq!(sublinhado_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), Sublinhado::Simples);
        let _ = app.update(Msg::Teclado(id, keyboard::Key::Character("u".into()), ctrl));
        assert_eq!(sublinhado_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), Sublinhado::Nenhum);
        // Ctrl+Alt+Tab / Ctrl+Alt+Shift+Tab: recuo, como no Excel.
        let tab = keyboard::Key::Named(keyboard::key::Named::Tab);
        let _ = app.update(Msg::Teclado(id, tab.clone(), ctrl | keyboard::Modifiers::ALT));
        let _ = app.update(Msg::Teclado(id, tab.clone(), ctrl | keyboard::Modifiers::ALT));
        assert_eq!(recuo_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), 2);
        let _ = app.update(Msg::Teclado(id, tab, ctrl | keyboard::Modifiers::ALT | keyboard::Modifiers::SHIFT));
        assert_eq!(recuo_de(&app.workbook.aba(0).unwrap().sheet, 0, 0), 1);
    }
}
