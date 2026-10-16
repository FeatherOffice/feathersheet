//! Comentários (anotações) das células: aba Revisão, Inserir > Comentário,
//! botão direito e Limpar ▾. O dado mora no motor
//! (`feathersheet_core::planilha::sheet::comentarios`), com desfazer; aqui
//! ficam só o editor (diálogo pequeno, seção 5 do GUIA-JANELAS) e o que a
//! grade mostra (balão do mouse, Mostrar todos).
use super::*;
use feathersheet_core::planilha::sheet::comentarios::Comentario;
use iced::widget::text_editor;

/// Comentário aberto no editor.
pub(super) struct Estado {
    pub folha: usize,
    pub linha: u32,
    pub coluna: u32,
    pub autor: String,
    pub conteudo: text_editor::Content,
    /// A célula ainda não tinha comentário.
    pub novo: bool,
}

#[derive(Debug, Clone)]
pub enum Evento {
    /// Novo comentário na célula ativa (ou editar, se já existe — Excel).
    Novo,
    Editar,
    /// Excluir os comentários da seleção.
    Excluir,
    Anterior,
    Proximo,
    /// Mostrar/ocultar o comentário da célula ativa (sempre à mostra).
    MostrarOcultar,
    /// Mostrar todos os comentários (só a tela, não o arquivo).
    MostrarTodos,
    /// Limpar ▾ > Limpar comentários (seleção).
    Limpar,
    Acao(text_editor::Action),
    Salvar,
    Cancelar,
    /// Célula com comentário sob o mouse (a grade só avisa quando muda).
    Sobre(Option<(u32, u32)>),
}

/// O nome de quem usa o computador: nome completo do sistema quando houver
/// (campo GECOS do `/etc/passwd`), senão o login. Nada sai da máquina.
pub(crate) fn autor_do_sistema() -> String {
    let login = std::env::var("USER").or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("LOGNAME")).unwrap_or_default();
    #[cfg(unix)]
    if !login.is_empty() {
        if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
            let nome = passwd.lines()
                .map(|l| l.split(':').collect::<Vec<_>>())
                .find(|campos| campos.first() == Some(&login.as_str()))
                .and_then(|campos| campos.get(4).map(|g| g.split(',').next().unwrap_or("").trim().to_string()))
                .filter(|n| !n.is_empty());
            if let Some(nome) = nome { return nome; }
        }
    }
    login
}

impl App {
    pub(super) fn visao_comentarios(&self) -> crate::grade::VisaoComentarios {
        crate::grade::VisaoComentarios {
            sobre: self.comentario_sobre,
            todos: self.comentarios_todos,
            editando: self.comentario_editor.as_ref()
                .filter(|e| e.folha == self.folha).map(|e| (e.linha, e.coluna)),
        }
    }

    pub(super) fn comentario_ativo(&self) -> Option<&Comentario> {
        self.workbook.aba(self.folha)?.sheet.comentario(self.linha, self.coluna)
    }

    fn aba_tem_comentarios(&self) -> bool {
        self.workbook.aba(self.folha).is_some_and(|a| !a.sheet.comentarios.is_empty())
    }

    fn abrir_editor_comentario(&mut self) -> Task<Msg> {
        if !self.confirmar() { return Task::none(); }
        let existente = self.comentario_ativo().cloned();
        let autor = existente.as_ref().map(|c| c.autor.clone()).unwrap_or_else(|| self.autor());
        self.comentario_editor = Some(Estado {
            folha: self.folha, linha: self.linha, coluna: self.coluna, autor,
            conteudo: text_editor::Content::with_text(existente.as_ref().map_or("", |c| c.texto.as_str())),
            novo: existente.is_none(),
        });
        self.menu_aberto = false;
        self.contexto = None;
        self.edicao_menu = None;
        focar_editor_comentario()
    }

    pub(super) fn evento_comentario(&mut self, evento: Evento) -> Task<Msg> {
        match evento {
            Evento::Novo | Evento::Editar => return self.abrir_editor_comentario(),
            Evento::Acao(acao) => if let Some(e) = &mut self.comentario_editor { e.conteudo.perform(acao); },
            Evento::Cancelar => self.comentario_editor = None,
            Evento::Salvar => {
                let Some(e) = self.comentario_editor.take() else { return Task::none() };
                // O text_editor sempre termina com uma quebra de linha.
                let texto = e.conteudo.text().trim_end_matches('\n').to_string();
                let visivel = self.workbook.aba(e.folha)
                    .and_then(|a| a.sheet.comentario(e.linha, e.coluna)).is_some_and(|c| c.visivel);
                let novo = Comentario { autor: e.autor, texto, visivel };
                let mut mudou = false;
                self.workbook.editar(e.folha, |s| mudou = s.definir_comentario(e.linha, e.coluna, Some(novo)));
                if mudou {
                    self.alterado = true;
                    self.status = format!("Comentário {} em {}", if e.novo { "inserido" } else { "editado" }, cell_name(e.linha, e.coluna));
                }
            }
            Evento::Excluir | Evento::Limpar => {
                self.confirmar();
                let areas = self.areas_selecionadas();
                let mut total = 0;
                self.workbook.editar(self.folha, |s| {
                    s.abrir_transacao();
                    for a in &areas { total += s.limpar_comentarios(a); }
                    s.fechar_transacao();
                });
                self.edicao_menu = None;
                if total > 0 {
                    self.alterado = true;
                    self.comentario_sobre = None;
                    self.status = if total == 1 { "Comentário excluído".into() }
                        else { format!("{total} comentários excluídos") };
                } else {
                    self.status = "A seleção não tem comentários".into();
                }
            }
            Evento::Anterior | Evento::Proximo => {
                let voltar = matches!(evento, Evento::Anterior);
                let alvo = self.workbook.aba(self.folha)
                    .and_then(|a| a.sheet.comentario_vizinho(self.linha, self.coluna, voltar));
                match alvo {
                    Some((r, c)) => {
                        self.confirmar();
                        self.trocar_selecao(r, c);
                        // O balão do comentário escolhido fica à vista até o mouse passar.
                        self.comentario_sobre = Some((r, c));
                        self.status = format!("Comentário em {}", cell_name(r, c));
                    }
                    None => self.status = "Esta planilha não tem comentários".into(),
                }
            }
            Evento::MostrarOcultar => {
                let Some(atual) = self.comentario_ativo() else {
                    self.status = "A célula ativa não tem comentário".into();
                    return Task::none();
                };
                let visivel = !atual.visivel;
                let mut area = self.area_selecionada();
                if !(area.r0..=area.r1).contains(&self.linha) || !(area.c0..=area.c1).contains(&self.coluna) {
                    area = Rect::cell(self.linha, self.coluna);
                }
                let mut n = 0;
                self.workbook.editar(self.folha, |s| n = s.mostrar_comentarios(&area, visivel));
                if n > 0 {
                    self.alterado = true;
                    self.status = if visivel { "Comentário sempre à mostra".into() } else { "Comentário oculto".into() };
                }
            }
            Evento::MostrarTodos => {
                self.comentarios_todos = !self.comentarios_todos;
                self.status = if self.comentarios_todos { "Mostrando todos os comentários".into() }
                    else { "Comentários ocultos (aparecem ao passar o mouse)".into() };
            }
            Evento::Sobre(celula) => self.comentario_sobre = celula,
        }
        Task::none()
    }

    /// Itens do botão direito na célula, como no Excel.
    pub(super) fn opcoes_contexto_comentario(&self) -> Vec<Option<(String, Msg)>> {
        let m = |e: Evento| Msg::Comentario(e);
        match self.comentario_ativo() {
            Some(c) => vec![
                Some(("Editar comentário".into(), m(Evento::Editar))),
                Some(("Excluir comentário".into(), m(Evento::Excluir))),
                Some((if c.visivel { "Ocultar comentário" } else { "Mostrar comentário" }.into(), m(Evento::MostrarOcultar))),
            ],
            None => vec![Some(("Inserir comentário".into(), m(Evento::Novo)))],
        }
    }

    /// Grupos da aba Revisão.
    pub(super) fn grupos_revisao(&self) -> Vec<Element<'_, Msg>> {
        let m = |e: Evento| Some(Msg::Comentario(e));
        let tem = self.comentario_ativo().is_some();
        let algum = self.aba_tem_comentarios();
        let principal = if tem {
            self.faixa_grande("Editar comentário", "edit", m(Evento::Editar), self.comentario_editor.is_some())
        } else {
            self.faixa_grande("Novo comentário", "message-circle", m(Evento::Novo), self.comentario_editor.is_some())
        };
        vec![
            self.faixa_grupo("Comentários", self.faixa_linha(vec![
                principal,
                self.faixa_pilha(vec![
                    self.faixa_pequeno("Excluir", "trash", tem.then(|| Msg::Comentario(Evento::Excluir)), false),
                    self.faixa_pequeno("Anterior", "chevron-left", algum.then(|| Msg::Comentario(Evento::Anterior)), false),
                    self.faixa_pequeno("Próximo", "chevron-right", algum.then(|| Msg::Comentario(Evento::Proximo)), false),
                ]),
                self.faixa_pilha(vec![
                    self.faixa_pequeno("Mostrar/ocultar comentário", "eye",
                        tem.then(|| Msg::Comentario(Evento::MostrarOcultar)),
                        self.comentario_ativo().is_some_and(|c| c.visivel)),
                    self.faixa_pequeno("Mostrar todos os comentários", "list-details",
                        m(Evento::MostrarTodos), self.comentarios_todos),
                ]),
            ])),
            self.faixa_grupo("Proteger", self.faixa_linha(vec![
                self.faixa_grande("Proteger planilha", "lock", Some(Msg::Protecao(protecao_ui::Evento::Abrir)),
                    self.protecao_modal.is_some()),
            ])),
        ]
    }

    fn view_comentario(&self) -> Element<'_, Msg> {
        use iced::widget::column;
        use iced::widget::text_editor::Binding;
        let e = self.comentario_editor.as_ref().expect("editor de comentário aberto");
        let titulo = if e.novo { "Novo comentário" } else { "Editar comentário" };
        let autor = if e.autor.is_empty() { "Sem autor".to_string() } else { e.autor.clone() };
        let editor = iced::widget::text_editor(&e.conteudo)
            .placeholder("Escreva o comentário")
            .on_action(|a| Msg::Comentario(Evento::Acao(a)))
            .key_binding(|tecla| {
                use iced::keyboard::{key::Named, Key};
                match tecla.key {
                    // Ctrl+Enter grava; Enter sozinho quebra a linha.
                    Key::Named(Named::Enter) if tecla.modifiers.control() =>
                        Some(Binding::Custom(Msg::Comentario(Evento::Salvar))),
                    Key::Named(Named::Escape) => Some(Binding::Custom(Msg::Comentario(Evento::Cancelar))),
                    _ => Binding::from_key_press(tecla),
                }
            })
            .height(Length::Fixed(132.0))
            .size(13)
            .padding(9);
        let mut rodape = row![].spacing(10).align_y(Alignment::Center);
        if !e.novo {
            rodape = rodape.push(button(row![feather_ui::icones::icone("trash", 15.), text("Excluir comentário").size(13)]
                .spacing(6).align_y(Alignment::Center))
                .on_press(Msg::Comentario(Evento::Excluir)).style(estilo_botao_secundario));
        }
        rodape = rodape.push(Space::with_width(Length::Fill))
            .push(button(text("Cancelar").size(13)).on_press(Msg::Comentario(Evento::Cancelar)).style(estilo_botao_secundario))
            .push(button(text("Salvar").size(13)).on_press(Msg::Comentario(Evento::Salvar)).style(estilo_botao_primario));
        let painel = column![
            text(titulo).size(18).font(iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT }),
            row![
                // As iniciais do autor (Primeiros passos ▸ Seus dados), num círculo.
                container(text(self.iniciais_do_autor(&e.autor)).size(10).font(iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT }))
                    .center_x(Length::Fixed(22.)).center_y(Length::Fixed(22.))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(COR_AZUL_FUNDO())),
                        border: iced::Border { radius: 11.0.into(), ..Default::default() },
                        ..Default::default()
                    }),
                text(format!("Célula {} · {autor}", cell_name(e.linha, e.coluna))).size(12).color(COR_TEXTO_MUTED()),
            ].spacing(7).align_y(Alignment::Center),
            container(editor).id(iced::widget::container::Id::new(ID_EDITOR)),
            text("Enter quebra a linha · Ctrl+Enter salva · Esc cancela").size(11).color(COR_TEXTO_MUTED()),
            rodape,
        ].spacing(10);
        let largura = (self.tamanho_principal.width - 24.0).clamp(300.0, 460.0);
        container(painel).width(Length::Fixed(largura)).padding(22).style(estilo_painel_faixa).into()
    }

    pub(super) fn sobrepor_comentario<'a>(&'a self, base: Element<'a, Msg>) -> Element<'a, Msg> {
        if self.comentario_editor.is_none() { return base; }
        let fora = iced::widget::mouse_area(Space::new(Length::Fill, Length::Fill))
            .on_press(Msg::Comentario(Evento::Cancelar));
        let janela = container(iced::widget::opaque(self.view_comentario()))
            .center_x(Length::Fill).center_y(Length::Fill)
            .width(Length::Fill).height(Length::Fill);
        iced::widget::stack![base, fora, janela].height(Length::Fill).into()
    }
}

impl App {
    /// Modo de captura: `B2=texto` (comentário), `D4!=texto` (sempre à
    /// mostra), `sobre:B2` (balão do mouse), `todos` (Mostrar todos) e
    /// `editor` (abre o editor na célula ativa). Itens separados por `|`.
    pub(super) fn preparar_captura_comentarios(&mut self, lista: &str) {
        use feathersheet_core::celulas::refs::parse_cell_ref;
        for item in lista.split('|') {
            if item == "editor" { let _ = self.abrir_editor_comentario(); continue; }
            if item == "todos" { self.comentarios_todos = true; continue; }
            if let Some(celula) = item.strip_prefix("sobre:").and_then(parse_cell_ref) {
                self.comentario_sobre = Some(celula);
                continue;
            }
            let Some((endereco, texto)) = item.split_once('=') else { continue };
            let (endereco, visivel) = match endereco.strip_suffix('!') { Some(e) => (e, true), None => (endereco, false) };
            if let Some((r, c)) = parse_cell_ref(endereco) {
                let nota = Comentario { autor: self.autor(), texto: texto.replace("\\n", "\n"), visivel };
                self.workbook.editar(self.folha, |s| { s.definir_comentario(r, c, Some(nota)); });
            }
        }
    }
}

const ID_EDITOR: &str = "feathersheet.comentario";

/// Põe o cursor no editor do comentário ao abrir (Iced 0.13 não tem
/// `text_editor::focus(id)`; mesma técnica do `campo_fx::focar_multilinha`).
fn focar_editor_comentario() -> Task<Msg> {
    use iced::advanced;
    struct Focar { dentro: bool, focou: bool }
    impl advanced::widget::Operation<Msg> for Focar {
        fn container(&mut self, id: Option<&advanced::widget::Id>, _bounds: iced::Rectangle,
            filhos: &mut dyn FnMut(&mut dyn advanced::widget::Operation<Msg>)) {
            let anterior = self.dentro;
            if id == Some(&advanced::widget::Id::new(ID_EDITOR)) { self.dentro = true; }
            filhos(self);
            self.dentro = anterior;
        }
        fn focusable(&mut self, estado: &mut dyn advanced::widget::operation::Focusable,
            _id: Option<&advanced::widget::Id>) {
            if self.dentro && !self.focou { estado.focus(); self.focou = true; }
            else { estado.unfocus(); }
        }
    }
    advanced::widget::operate(Focar { dentro: false, focou: false })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn comentario(app: &App, r: u32, c: u32) -> Option<Comentario> {
        app.workbook.aba(0).unwrap().sheet.comentario(r, c).cloned()
    }

    fn escrever(app: &mut App, texto: &str) {
        let _ = app.update(Msg::Comentario(Evento::Acao(text_editor::Action::SelectAll)));
        for ch in texto.chars() {
            let _ = app.update(Msg::Comentario(Evento::Acao(text_editor::Action::Edit(
                text_editor::Edit::Insert(ch)))));
        }
    }

    #[test]
    fn inserir_editar_excluir_e_desfazer_pela_interface() {
        let mut app = App::default();
        let _ = app.update(Msg::Comentario(Evento::Novo));
        assert!(app.comentario_editor.as_ref().is_some_and(|e| e.novo));
        escrever(&mut app, "Conferir");
        let _ = app.update(Msg::Comentario(Evento::Salvar));
        assert!(app.comentario_editor.is_none());
        let c = comentario(&app, 0, 0).expect("comentário gravado");
        assert_eq!((c.texto.as_str(), c.autor), ("Conferir", autor_do_sistema()));
        assert!(app.alterado);

        // Novo numa célula com comentário edita o mesmo.
        let _ = app.update(Msg::Comentario(Evento::Novo));
        assert!(app.comentario_editor.as_ref().is_some_and(|e| !e.novo));
        escrever(&mut app, "Outro");
        let _ = app.update(Msg::Comentario(Evento::Salvar));
        assert_eq!(comentario(&app, 0, 0).unwrap().texto, "Outro");

        let _ = app.update(Msg::Comentario(Evento::MostrarOcultar));
        assert!(comentario(&app, 0, 0).unwrap().visivel);
        let _ = app.update(Msg::Comentario(Evento::Excluir));
        assert!(comentario(&app, 0, 0).is_none());
        let _ = app.update(Msg::Desfazer);
        assert!(comentario(&app, 0, 0).unwrap().visivel);
        let _ = app.update(Msg::Desfazer);
        let _ = app.update(Msg::Desfazer);
        assert_eq!(comentario(&app, 0, 0).unwrap().texto, "Conferir");
    }

    #[test]
    fn cancelar_nao_grava_e_proximo_anda_pela_aba() {
        let mut app = App::default();
        let _ = app.update(Msg::Comentario(Evento::Novo));
        escrever(&mut app, "x");
        let _ = app.update(Msg::Comentario(Evento::Cancelar));
        assert!(comentario(&app, 0, 0).is_none());

        app.workbook.editar(0, |s| {
            for (r, c) in [(4, 1), (9, 3)] {
                s.definir_comentario(r, c, Some(Comentario { autor: "A".into(), texto: "t".into(), visivel: false }));
            }
        });
        let _ = app.update(Msg::Comentario(Evento::Proximo));
        assert_eq!((app.linha, app.coluna), (4, 1));
        assert_eq!(app.comentario_sobre, Some((4, 1)));
        let _ = app.update(Msg::Comentario(Evento::Proximo));
        assert_eq!((app.linha, app.coluna), (9, 3));
        let _ = app.update(Msg::Comentario(Evento::Proximo));
        assert_eq!((app.linha, app.coluna), (4, 1), "dá a volta");
        let _ = app.update(Msg::Comentario(Evento::Anterior));
        assert_eq!((app.linha, app.coluna), (9, 3));
    }

    #[test]
    fn limpar_comentarios_e_limpar_tudo_tiram_o_comentario() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 0, 0, "abc".into());
        app.workbook.editar(0, |s| {
            s.definir_comentario(0, 0, Some(Comentario { autor: String::new(), texto: "t".into(), visivel: false }));
        });
        let _ = app.update(Msg::Comentario(Evento::Limpar));
        assert!(comentario(&app, 0, 0).is_none());
        assert_eq!(app.workbook.aba(0).unwrap().sheet.get_raw(0, 0), "abc", "só o comentário sai");
        let _ = app.update(Msg::Desfazer);
        assert!(comentario(&app, 0, 0).is_some());
        let _ = app.update(Msg::LimparTudo);
        assert!(comentario(&app, 0, 0).is_none());
        assert_eq!(app.workbook.aba(0).unwrap().sheet.get_raw(0, 0), "");
        let _ = app.update(Msg::Desfazer);
        assert!(comentario(&app, 0, 0).is_some(), "Limpar tudo é um Ctrl+Z só");
        assert_eq!(app.workbook.aba(0).unwrap().sheet.get_raw(0, 0), "abc");
    }

    #[test]
    fn revisao_contexto_e_editor_montam() {
        let mut app = App::default();
        assert_eq!(app.opcoes_contexto_comentario().len(), 1);
        app.faixa_ativa = AbaFaixa::Revisao;
        assert_eq!(app.montar_grupos_faixa().len(), app.faixa_larguras().len());
        let _ = app.update(Msg::Comentario(Evento::Novo));
        let _: Element<'_, Msg> = app.view_comentario();
        let _ = app.update(Msg::Comentario(Evento::Salvar));
        assert_eq!(app.opcoes_contexto_comentario().len(), 3);
        let _ = app.update(Msg::Comentario(Evento::Sobre(Some((0, 0)))));
        assert_eq!(app.visao_comentarios().sobre, Some((0, 0)));
        let _ = app.update(Msg::Comentario(Evento::MostrarTodos));
        assert!(app.visao_comentarios().todos);
        for icone in ["message-circle", "edit", "trash", "chevron-left", "chevron-right", "eye", "list-details", "lock"] {
            assert!(feather_ui::icones::existe(icone), "{icone}");
        }
    }
}
