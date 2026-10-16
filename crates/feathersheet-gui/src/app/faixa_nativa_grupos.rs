//! Montagem dos grupos da Ribbon e renderização do menu de ferramentas extras.
use super::*;
use iced::widget::column;
use feather_ui::componentes as ui;
use super::faixa_responsiva::{self, Nivel};
use super::faixa_nativa::*;

impl App {
    /// A lista de grupos é única. Ribbon e menu Mais usam os mesmos widgets,
    /// sem duplicar nem reduzir funcionalidade de controles ocultos.
    pub(super) fn montar_grupos_faixa(&self) -> Vec<Element<'_, Msg>> {
        let estilo = self.faixa_estilo();
        let mut grupos: Vec<Element<'_, Msg>> = Vec::new();
        match self.faixa_ativa {
            AbaFaixa::Inicio => {
                grupos.push(self.faixa_grupo("Área de transferência", self.faixa_linha(vec![
                    self.faixa_botao_colar(),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno_colorido("Recortar", "cut", "#d68126", Some(Msg::Recortar), false),
                        self.faixa_pequeno_colorido("Copiar", "copy", "#2179a7", Some(Msg::Copiar), false),
                        self.faixa_pequeno_colorido("Pincel", "brush", "#7d5cc6",
                            Some(Msg::Pincel), self.pincel_formatacao.is_some()),
                    ]),
                ])));
                let tamanho = estilo.font_size.unwrap_or(13.).round().clamp(6., 96.) as i32;
                // Fonte compacta, em duas linhas na ordem de trabalho:
                // [família] [tamanho] [A+] [A−]
                // [B] [I] [Sublinhado] [Bordas ▾] [Balde/cor] [A/cor].
                // Tachado continua disponível no fim para não perder a função.
                let fonte_superior = row![
                    self.faixa_lista("Fonte", "fonte", FONTES, Some(self.faixa_nome_fonte()), 126.),
                    self.faixa_lista("Tamanho", "tamanho", TAMANHOS,
                        TAMANHOS.iter().copied().find(|v| v.parse::<i32>().ok() == Some(tamanho)), 68.),
                    self.faixa_tamanho_acao("A+", "Aumentar tamanho da fonte", 1),
                    self.faixa_tamanho_acao("A−", "Diminuir tamanho da fonte", -1),
                ]
                .spacing(5)
                .align_y(Alignment::Center);
                let fonte_inferior = row![
                    self.faixa_icone("Negrito (Ctrl+B)", "bold", Some(Msg::AlternarEstilo("bold".into())), estilo.bold),
                    self.faixa_icone("Itálico", "italic", Some(Msg::AlternarEstilo("italic".into())), estilo.italic),
                    self.faixa_sublinhado(estilo.underline),
                    self.faixa_bordas(),
                    self.faixa_cor(AlvoCor::Fundo),
                    self.faixa_cor(AlvoCor::Texto),
                    self.faixa_icone("Tachado", "strikethrough", Some(Msg::AlternarEstilo("strike".into())), estilo.strike),
                ]
                .spacing(4)
                .align_y(Alignment::Center);
                grupos.push(ui::grupo_faixa_compacto(
                    column![fonte_superior, fonte_inferior].spacing(6).into(),
                    "Fonte",
                ));
                grupos.push(self.faixa_grupo("Alinhamento", self.faixa_linha(vec![
                    self.faixa_pilha(vec![
                        feather_ui::alinhamento::seletor_horizontal(
                            match estilo.align {
                                feathersheet_core::celulas::style::Align::Auto => None,
                                feathersheet_core::celulas::style::Align::Left =>
                                    Some(feather_ui::alinhamento::Horizontal::Esquerda),
                                feathersheet_core::celulas::style::Align::Center =>
                                    Some(feather_ui::alinhamento::Horizontal::Centro),
                                feathersheet_core::celulas::style::Align::Right =>
                                    Some(feather_ui::alinhamento::Horizontal::Direita),
                            },
                            true,
                            false,
                            |escolha| Msg::EstiloMenu(
                                "align".into(),
                                Campo::Texto(match escolha {
                                    feather_ui::alinhamento::Horizontal::Esquerda => "left",
                                    feather_ui::alinhamento::Horizontal::Centro => "center",
                                    feather_ui::alinhamento::Horizontal::Direita => "right",
                                    // A faixa de células expõe apenas três botões;
                                    // nunca mapear Justificado erroneamente para Esquerda.
                                    feather_ui::alinhamento::Horizontal::Justificado => "justify",
                                }.into()),
                            ),
                        ),
                        self.faixa_linha(vec![
                            self.faixa_vertical("top", estilo.valign == feathersheet_core::celulas::style::VAlign::Top),
                            self.faixa_vertical("middle", estilo.valign == feathersheet_core::celulas::style::VAlign::Middle),
                            self.faixa_vertical("bottom", estilo.valign == feathersheet_core::celulas::style::VAlign::Bottom),
                            self.faixa_icone("Diminuir recuo", "indent-decrease", Some(Msg::AlterarRecuo(-1)), false),
                            self.faixa_icone("Aumentar recuo", "indent-increase", Some(Msg::AlterarRecuo(1)), false),
                        ]),
                    ]),
                    self.faixa_pilha(vec![
                        self.faixa_inclinar_texto(estilo.rotation.unwrap_or(0) != 0),
                        self.faixa_pequeno("Quebrar", "text-wrap",
                            Some(Msg::EstiloMenu("wrap".into(), Campo::Logico(!estilo.wrap))), estilo.wrap),
                    ]),
                    self.faixa_mesclar_dividido(
                        self.workbook.aba(self.folha).is_some_and(|aba|
                            mesclas::exatamente_mesclada(&aba.sheet, &self.area_selecionada()))),
                ])));
                // Número como no mockup: formato em cima; cinco ações compactas abaixo.
                // Cada botão conserva o comando original de formatação da planilha.
                let numero_inferior = row![
                    self.faixa_numero_botao("R$", "Formato de moeda",
                        Msg::EstiloMenu("format".into(), Campo::Texto("currency".into())),
                        estilo.format == feathersheet_core::celulas::style::NumberFormat::Currency),
                    self.faixa_numero_botao("%", "Formato percentual",
                        Msg::EstiloMenu("format".into(), Campo::Texto("percent".into())),
                        estilo.format == feathersheet_core::celulas::style::NumberFormat::Percent),
                    self.faixa_numero_botao(".000", "Formato numérico",
                        Msg::EstiloMenu("format".into(), Campo::Texto("number".into())),
                        estilo.format == feathersheet_core::celulas::style::NumberFormat::Number),
                    self.faixa_numero_botao(".0←", "Diminuir casas decimais",
                        Msg::Decimais(-1), false),
                    self.faixa_numero_botao(".0→", "Aumentar casas decimais",
                        Msg::Decimais(1), false),
                ].spacing(1).align_y(Alignment::Center);
                grupos.push(self.faixa_grupo(NUM_FORMAT,
                    column![
                        self.faixa_lista("Formato", "formato", FORMATOS,
                            Some(self.faixa_nome_formato()), 173.),
                        numero_inferior,
                    ].spacing(10).into(),
                ));
                // Ícones de destaque seguem a identidade de cor do mockup.
                // Não há ações fictícias: cada botão aponta para o comando já existente.
                grupos.push(self.faixa_grupo("Estilos", self.faixa_linha(vec![
                    self.faixa_grande_destaque(
                        "Formatação\ncondicional", "palette", "#7d5cc6",
                        (238, 232, 252), Some(Msg::CondicionalAbrir), self.condicional_menu_aberto,
                    ),
                    button(column![
                        feather_ui::icones::icone("table", 25.),
                        row![
                            text("Tabela").size(11),
                            feather_ui::icones::icone("chevron-down", 12.0),
                        ].spacing(3).align_y(Alignment::Center),
                    ].spacing(3).align_x(Alignment::Center))
                        .padding([4, 8])
                        .style(estilo_botao_grande(self.tabelas_menu_aberto))
                        .on_press(Msg::AlternarMenuTabelas).into(),
                    button(column![
                        feather_ui::icones::icone("cell-styles", 25.),
                        text("Estilos").size(11),
                        row![
                            text("de Célula").size(11),
                            feather_ui::icones::icone("chevron-down", 12.0),
                        ].spacing(3).align_y(Alignment::Center),
                    ].spacing(2).align_x(Alignment::Center))
                        .padding([3, 8])
                        .style(estilo_botao_grande(self.estilos_menu_aberto))
                        .on_press(Msg::AlternarMenuEstilos).into(),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno_colorido(
                            "Cores alternadas", "alternating-colors", "#7d5cc6",
                            Some(Msg::CondicionalCoresAlternadas), false,
                        ),
                        self.faixa_pequeno("Limpar formato", "eraser",
                            Some(Msg::LimparFormato), false),
                    ]),
                ])));
                // Operações estruturais não têm falsas setas de submenu:
                // a legenda explica a ação executada ao clicar.
                grupos.push(self.faixa_grupo("Células", self.faixa_linha(vec![
                    self.faixa_com_seta(self.faixa_grande_destaque(
                        "Inserir\nlinha", "plus", "#21865c",
                        (222, 245, 233),
                        Some(Msg::InserirMenu(Estrutura::InserirLinha, false)), false,
                    ), "inserir"),
                    self.faixa_com_seta(self.faixa_grande_destaque(
                        "Excluir\nlinha", "trash", "#ca5555",
                        (253, 232, 232),
                        Some(Msg::Estrutura(Estrutura::ExcluirLinha)), false,
                    ), "excluir"),
                    self.faixa_formatar_celulas_menu(),
                ])));
                // Edição: Σ AutoSoma/Preencher à esquerda e ações grandes à direita.
                // Os ícones de seta só abrem menus suspensos reais.
                grupos.push(self.faixa_grupo("Edição", self.faixa_linha(vec![
                    self.faixa_pilha(vec![
                        self.faixa_autosoma_compacto(),
                        // O cálculo de séries, fórmulas relativas e estilos já é Rust:
                        // botão dividido executa para baixo; a seta abre os demais modos.
                        self.faixa_preencher_compacto(),
                        self.faixa_limpar_compacto(),
                    ]),
                    self.faixa_classificar_filtrar(),
                    self.faixa_localizar_grande(),
                ])));
            }
                        AbaFaixa::Inserir => {
                // Primeiro a iconografia; comandos existentes seguem funcionais.
                grupos.push(self.faixa_grupo("Tabelas", self.faixa_linha(vec![
                    self.faixa_grande_seta("Tabela dinâmica", "pivot-table", Some(Msg::Dinamica(dinamica_ui::Evento::Abrir)), self.dinamica.is_some()),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Dinâmicas recomendadas", "table-suggest",
                            Some(Msg::Dinamica(dinamica_ui::Evento::Recomendadas)), false),
                        self.faixa_pequeno("Editar dinâmica", "edit",
                            Some(Msg::Dinamica(dinamica_ui::Evento::Editar)), false),
                        self.faixa_pequeno("Atualizar dinâmica", "refresh",
                            Some(Msg::Dinamica(dinamica_ui::Evento::Atualizar)), false),
                    ]),
                    self.faixa_grande("Tabela", "table", Some(Msg::TabelaAbrir), self.tabela_area.is_some()),
                ])));
                grupos.push(self.faixa_grupo("Ilustrações", self.faixa_linha(vec![
                    self.faixa_grande_seta("Imagens", "photo", Some(Msg::ImagemLaboratorioAbrir), false),
                    // Formas: menu flutuante com miniaturas; o arraste na grade cria (objetos.rs).
                    self.faixa_grande_seta("Formas", "shapes", Some(Msg::Objetos(super::objetos::Evento::AlternarMenu(super::objetos::ModoMenu::Inserir))),
                        matches!(self.objetos.menu, Some(super::objetos::ModoMenu::Inserir | super::objetos::ModoMenu::Trocar)) || self.objetos.inserir.is_some()),
                    // Ícones: galeria Tabler com busca; um clique insere (objetos.rs).
                    self.faixa_grande("Ícones", "icons", Some(Msg::Objetos(super::objetos::Evento::AlternarMenu(super::objetos::ModoMenu::Icones))),
                        self.objetos.menu == Some(super::objetos::ModoMenu::Icones)),
                    em_breve(self.faixa_grande_seta("Modelos 3D", "cube", None, false), "Em breve: modelos 3D"),
                    em_breve(self.faixa_grande("SmartArt", "sitemap", None, false), "Em breve: diagramas SmartArt"),
                    em_breve(self.faixa_grande("Instantâneo", "screenshot", None, false), "Em breve: captura de outra janela"),
                    self.faixa_pequeno("Efeitos da imagem", "photo", self.workbook.aba(self.folha).and_then(|aba| aba.sheet.im.as_ref()).map(|_| Msg::ImagemEfeitosAbrir), self.efeitos_imagem.aberto),
                ])));
                grupos.push(self.faixa_grupo("Controles", self.faixa_linha(vec![
                    // Caixas de seleção: insere na seleção; aceso = a seleção tem caixa
                    // e o clique remove (caixas_ui.rs).
                    self.faixa_grande("Checkbox", "checkbox", Some(Msg::CaixasInserir), self.selecao_tem_caixa()),
                ])));
                grupos.push(self.faixa_grupo("Gráficos", self.faixa_linha(vec![
                    self.faixa_grande("Recomendados", "chart-suggest", None, false),
                    self.faixa_grafico(),
                    self.faixa_pilha(vec![
                        self.faixa_linha(vec![
                    self.faixa_icone("Colunas", "chart-columns", None, false),
                    self.faixa_icone("Linhas", "chart-line", None, false),
                    self.faixa_icone("Pizza", "chart-pie", None, false),
                    self.faixa_icone("Área", "chart-area", None, false),
                ]),
                        self.faixa_linha(vec![
                    self.faixa_icone("Dispersão", "chart-scatter", None, false),
                    self.faixa_icone("Combinado", "chart-combo", None, false),
                    self.faixa_icone("Cascata", "chart-waterfall", None, false),
                ]),
                    ]),
                    self.faixa_grande_seta("Mapas", "chart-map", None, false),
                    self.faixa_grande_seta("Gráfico dinâmico", "chart-pivot", None, false),
                ])));
                grupos.push(self.faixa_grupo("Minigráficos", self.faixa_linha(vec![
                    em_breve(self.faixa_grande("Linha", "sparkline-line", None, false), "Em breve: minigráfico de linha"),
                    em_breve(self.faixa_grande("Coluna", "sparkline-columns", None, false), "Em breve: minigráfico de colunas"),
                    em_breve(self.faixa_grande("Ganhos/Perdas", "sparkline-winloss", None, false), "Em breve: minigráfico de ganhos e perdas"),
                ])));
                grupos.push(self.faixa_grupo("Filtros", self.faixa_linha(vec![
                    em_breve(self.faixa_grande("Segmentação de dados", "slicer", None, false), "Em breve: segmentação de dados (filtro visual)"),
                    em_breve(self.faixa_grande("Linha do tempo", "timeline", None, false), "Em breve: linha do tempo de datas"),
                ])));
                grupos.push(self.faixa_grupo("Links", self.faixa_linha(vec![
                    self.faixa_grande("Inserir link", "link", Some(Msg::LinkEditar),
                        self.link_modal.is_some()),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Abrir link", "link",
                            self.link_da_celula(self.folha, self.linha, self.coluna)
                                .map(|_| Msg::LinkAbrir), false),
                        self.faixa_pequeno("Remover link", "trash",
                            Some(Msg::LinkRemover), false),
                    ]),
                ])));
                grupos.push(self.faixa_grupo("Comentários", self.faixa_linha(vec![
                    self.faixa_grande("Comentário", "message-circle",
                        Some(Msg::Comentario(comentarios_ui::Evento::Novo)), self.comentario_editor.is_some()),
                ])));
                grupos.push(self.faixa_grupo("Texto", self.faixa_linha(vec![
                    self.faixa_grande("Caixa de texto", "text-box", None, false),
                    self.faixa_grande("Cabeçalho e rodapé", "header-footer", None, false),
                    // WordArt: galeria dos estilos comuns; um clique insere (objetos.rs).
                    self.faixa_grande("WordArt", "wordart", Some(Msg::Objetos(super::objetos::Evento::AlternarMenu(super::objetos::ModoMenu::WordArt))),
                        self.objetos.menu == Some(super::objetos::ModoMenu::WordArt)),
                    self.faixa_grande("Linha de assinatura", "signature", None, false),
                    self.faixa_grande("Objeto", "object", None, false),
                ])));
                grupos.push(self.faixa_grupo("Símbolos", self.faixa_linha(vec![
                    self.faixa_grande("Equação", "equation", None, false),
                    self.faixa_grande("Símbolo", "omega", None, false),
                ])));
                // Comandos já presentes em Inserir: preservados no final.
                grupos.push(self.faixa_grupo("Células", self.faixa_pilha(vec![
                        self.faixa_linha(vec![
                    self.faixa_lista("Linhas", "linhas", LINHAS, None, 136.),
                    self.faixa_lista("Colunas", "colunas", COLUNAS, None, 150.),
                ]),
                        self.faixa_linha(vec![
                    self.faixa_pequeno("Nova aba", "file-plus", Some(Msg::AdicionarAba), false),
                    self.faixa_pequeno("Excluir linha", "trash", Some(Msg::Estrutura(Estrutura::ExcluirLinha)), false),
                    self.faixa_pequeno("Excluir coluna", "trash", Some(Msg::Estrutura(Estrutura::ExcluirColuna)), false),
                ]),
                    ])));
                grupos.push(self.faixa_grupo("Fórmulas", self.faixa_linha(vec![
                    self.faixa_lista("Função", "funcao", FUNCOES, None, 100.),
                    self.faixa_grande("Nomear intervalo", "tag", Some(Msg::NomesAbrir(true)), self.nomes_modal.is_some()),
                    self.faixa_catalogo_funcoes(),
                    self.faixa_grande("Explicar erro", "alerta_triangulo", Some(Msg::ExplicacaoErroAbrir), false),
                ])));
            }
            AbaFaixa::Desenhar => grupos.extend(
                feather_ui::tinta::grupos_faixa(&self.tinta.ui).into_iter().map(|g| g.map(Msg::Desenhar)),
            ),
            AbaFaixa::Formulas => grupos.extend(self.grupos_formulas()),
            AbaFaixa::Revisao => grupos.extend(self.grupos_revisao()),
            // Contextual: só com forma selecionada (objetos/interface.rs).
            AbaFaixa::EditarForma => grupos.extend(self.grupos_editar_forma()),
            AbaFaixa::Dados => {
                grupos.push(self.faixa_grupo("Classificar e filtrar", self.faixa_linha(vec![
                    self.faixa_grande("AutoFiltro", "filter",
                        Some(Msg::TabelaAcao(tabelas::Acao::AlternarAutoFiltro)), false),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("A–Z", "sort-ascending", Some(Msg::Ordenar(true)), false),
                        self.faixa_pequeno("Z–A", "sort-descending", Some(Msg::Ordenar(false)), false),
                        self.faixa_lista("Classificar página", "ordenar", ORDENAR, None, 140.),
                    ]),
                ])));
                grupos.push(self.faixa_grupo("Ferramentas de dados", self.faixa_linha(vec![
                    // Ação principal abre a janela diretamente: sem menu intermediário.
                    self.faixa_grande("Dividir texto", "columns",
                        Some(Msg::AbrirAssistenteDivisao), self.assistente_divisao.is_some()),
                    self.faixa_grande("Duplicados", "copy", Some(Msg::RemoverDuplicados), false),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Remover espaços", "eraser", Some(Msg::ApararEspacos), false),
                        self.faixa_pequeno("Estatísticas", "chart-bar", Some(Msg::EstatisticasAbrir), self.estatisticas.is_some()),
                        self.faixa_pequeno("Perfil da coluna", "chart-bar", Some(Msg::PerfilColunaAbrir), self.perfil_coluna.is_some()),
                    ]),
                    self.faixa_pilha(vec![
                        self.faixa_validacao(),
                        self.faixa_pequeno("Nomes", "tag", Some(Msg::NomesAbrir(false)), self.nomes_modal.is_some()),
                    ]),
                    // Preenchimento relâmpago (Ctrl+E): rótulo curto para a
                    // faixa caber em 1280 px sem espremer o grupo seguinte.
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Relâmpago", "bolt", Some(Msg::PreenchimentoRelampago), false),
                        self.faixa_pequeno("Região atual", "table", Some(Msg::SelecionarRegiaoAtual), false),
                    ]),
                ])));
                grupos.push(self.grupo_estrutura_topicos());
                grupos.push(self.faixa_grupo("Previsão", self.faixa_linha(vec![
                    self.faixa_grande("Cenários", "list-details",
                        Some(Msg::Cenarios(cenarios_ui::Evento::Abrir)), self.cenarios.is_some()),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Atingir meta…", "math-function",
                            Some(Msg::AtingirMeta(atingir_meta::Evento::Abrir)), self.atingir_meta.is_some()),
                        self.faixa_pequeno("Tabela de dados…", "table",
                            Some(Msg::TabelaDados(tabela_dados::Evento::Abrir)), self.tabela_dados.is_some()),
                        // Sem tabela feita nesta sessão, fica desabilitado.
                        self.faixa_pequeno("Atualizar tabela", "refresh",
                            self.tabela_dados_ultima.is_some()
                                .then_some(Msg::TabelaDados(tabela_dados::Evento::Atualizar)), false),
                    ]),
                ])));
                // Design na aba contextual; filtros/ordenação na seta do cabeçalho.
                grupos.push(self.faixa_grupo("Consultas", self.faixa_linha(vec![
                    self.faixa_grande("OpenQuery", "table", Some(Msg::OpenQuery(EventoOpenQuery::Abrir)), self.openquery.is_some()),
                    self.faixa_grande("Tabela dinâmica", "pivot-table", Some(Msg::Dinamica(dinamica_ui::Evento::Abrir)), self.dinamica.is_some()),
                    self.faixa_grande("Atualizar vínculos", "refresh", None, false),
                ])));
            }
            AbaFaixa::Exibir => {
                grupos.push(self.faixa_grupo("Mostrar", self.faixa_linha(vec![
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Linhas de grade", "table", Some(Msg::LinhasGrade(!self.linhas_grade_visiveis())), self.linhas_grade_visiveis()),
                        self.faixa_pequeno("Barra de fórmulas", "math-function", Some(Msg::AlternarBarraFx), self.mostrar_fx),
                        self.faixa_pequeno("Fórmulas", "code", Some(Msg::AlternarMostrarFormulas), self.mostrar_formulas),
                    ]),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Pintar linha/coluna", "palette", Some(Msg::RealceEixos(!self.config.realce_eixos)), self.config.realce_eixos),
                        self.faixa_lista("Tema", "tema", TEMA, Some(if self.config.escuro { "Escuro" } else { "Claro" }), 91.),
                        self.faixa_pequeno("Monitor", "device-desktop", Some(Msg::Monitor), false),
                    ]),
                ])));
                // Tamanho 0 no mesmo mapa de dimensões do TypeScript.
                // Opera na seleção e fica disponível também no menu de cabeçalhos.
                grupos.push(self.faixa_grupo("Linhas e colunas", self.faixa_linha(vec![
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Ocultar linhas", "eye-off",
                            Some(Msg::OcultarDimensoes(false)), false),
                        self.faixa_pequeno("Reexibir linhas", "inspecao",
                            Some(Msg::ReexibirDimensoes(false)), false),
                    ]),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Ocultar colunas", "eye-off",
                            Some(Msg::OcultarDimensoes(true)), false),
                        self.faixa_pequeno("Reexibir colunas", "inspecao",
                            Some(Msg::ReexibirDimensoes(true)), false),
                    ]),
                ])));
                grupos.push(self.faixa_grupo("Janela", self.faixa_linha(vec![
                    self.faixa_grande_seta("Congelar", "lock", Some(Msg::CongelarMenu), self.congelar_menu_aberto),
                    self.faixa_grande(if self.tela_inteira { "Sair tela inteira" } else { "Tela inteira" },
                        "arrows-maximize", Some(Msg::TelaInteira), self.tela_inteira),
                ])));
                grupos.push(self.faixa_grupo("Zoom", self.faixa_lista("Zoom", "zoom", ZOOMS, self.faixa_nome_zoom(), 88.)));
                grupos.push(self.faixa_grupo("Impressão", self.faixa_linha(vec![
                    self.faixa_grande("Visualizar", "printer", Some(Msg::Impressao(EventoImpressao::Abrir)), false),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Definir área", "square", Some(Msg::Impressao(EventoImpressao::DefinirArea)), false),
                        self.faixa_pequeno("Limpar área", "eraser", Some(Msg::Impressao(EventoImpressao::LimparArea)), false),
                        self.faixa_pequeno("Quebras de página", "file", Some(Msg::Impressao(EventoImpressao::AlternarQuebras)), self.quebras_mostradas()),
                    ]),
                ])));
            }
            AbaFaixa::Macros => {
                grupos.push(self.faixa_grupo("Editor", self.faixa_linha(vec![
                    self.faixa_grande("Editor de macros", "code", Some(Msg::Macros), false),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Novo módulo", "plus", Some(Msg::MacroNovoModulo), false),
                        self.faixa_pequeno("Novo componente", "plus", None, false),
                        self.faixa_pequeno("Editar", "edit", Some(Msg::Macros), false),
                    ]),
                ])));
                grupos.push(self.faixa_grupo("Executar", self.faixa_linha(vec![
                    self.faixa_grande("Executar macro", "player-play", Some(Msg::MacroExecutar), false),
                    self.faixa_grande("Componentes", "componentes", None, false),
                ])));
                grupos.push(self.faixa_grupo("Planilha", self.faixa_grande("Botão de macro", "code", Some(Msg::BotaoMacroNovo), false)));
                grupos.push(self.faixa_grupo("Ajuda", self.faixa_grande("Guia das funções", "guia_funcoes", Some(Msg::Ajuda), false)));
            }
            AbaFaixa::Tabela => {
                // Paridade de src/ui/barras/ribbon-tabela.ts.
                if let Some(t)=tabelas::sob_celula(
                    &self.workbook,self.folha,self.linha,self.coluna)
                {
                    let acao_nome = if self.tabela_renomeando {
                        self.faixa_linha(vec![
                            text_input("Novo nome",&self.tabela_nome_novo)
                                .on_input(|v|Msg::TabelaAcao(tabelas::Acao::Nome(v)))
                                .on_submit(Msg::TabelaAcao(tabelas::Acao::Renomear))
                                .width(Length::Fixed(154.0)).padding(6).into(),
                            self.faixa_pequeno("Salvar","confirmar",
                                Some(Msg::TabelaAcao(tabelas::Acao::Renomear)),false),
                            self.faixa_pequeno("Cancelar","fechar",
                                Some(Msg::TabelaAcao(tabelas::Acao::CancelarRenomeacao)),false),
                        ])
                    } else {
                        self.faixa_linha(vec![
                            self.faixa_pequeno("Renomear","edit",
                                Some(Msg::TabelaAcao(tabelas::Acao::IniciarRenomeacao)),false),
                        ])
                    };
                    let acao_intervalo = if self.tabela_redimensionando {
                        self.faixa_linha(vec![
                            text_input("A1:C20",&self.tabela_intervalo_novo)
                                .on_input(|v|Msg::TabelaAcao(tabelas::Acao::Intervalo(v)))
                                .on_submit(Msg::TabelaAcao(tabelas::Acao::Redimensionar))
                                .width(Length::Fixed(116.0)).padding(6).into(),
                            self.faixa_pequeno("Salvar","confirmar",
                                Some(Msg::TabelaAcao(tabelas::Acao::Redimensionar)),false),
                            self.faixa_pequeno("Cancelar","fechar",
                                Some(Msg::TabelaAcao(tabelas::Acao::CancelarRedimensionamento)),false),
                        ])
                    } else {
                        self.faixa_linha(vec![
                            self.faixa_pequeno("Redimensionar","table",
                                Some(Msg::TabelaAcao(tabelas::Acao::IniciarRedimensionamento)),false),
                            self.faixa_pequeno("Converter","table",
                                Some(Msg::TabelaAcao(tabelas::Acao::Converter)),false),
                        ])
                    };
                    grupos.push(self.faixa_grupo("Propriedades", self.faixa_pilha(vec![
                        acao_nome, acao_intervalo,
                    ])));
                    grupos.push(self.faixa_grupo("Opções de estilo",self.faixa_pilha(vec![
                        self.faixa_linha(vec![
                            self.faixa_pequeno("Faixas por linha","layout-rows",
                                Some(Msg::TabelaAcao(tabelas::Acao::Faixas)),t["faixas"]==true),
                            self.faixa_pequeno("Faixas por coluna","layout-columns",
                                Some(Msg::TabelaAcao(tabelas::Acao::FaixasColuna)),
                                t["estilo"]["faixasColuna"]==true),
                        ]),
                        self.faixa_linha(vec![
                            self.faixa_pequeno("Bordas","border-all",
                                Some(Msg::TabelaAcao(tabelas::Acao::Borda)),
                                t["estilo"]["borda"]==true),
                            self.faixa_pequeno("Linha de totais","sum",
                                Some(Msg::TabelaAcao(tabelas::Acao::Totais)),
                                t["mostrarTotais"]==true),
                        ]),
                    ])));
                    // Igual ao TS: um botão abre o menu suspenso (CRIAR TABELA +
                    // grade de miniaturas), o mesmo da aba Início. Sem paleta em bolinhas.
                    let botao_estilos = button(column![
                        estilos_popup::icone_tabela(),
                        row![
                            text("Estilos de tabela").size(11),
                            feather_ui::icones::icone("chevron-down", 12.0),
                        ].spacing(3).align_y(Alignment::Center),
                    ].spacing(3).align_x(Alignment::Center))
                        .padding([4, 8])
                        .style(estilo_botao_grande(self.tabelas_menu_aberto))
                        .on_press(Msg::AlternarMenuTabelas);
                    grupos.push(self.faixa_grupo("Estilos de Tabela", botao_estilos.into()));
                } else {
                    grupos.push(self.faixa_grupo("Tabela",
                        text("Selecione uma célula da tabela").size(12).into()));
                }
            }
            AbaFaixa::Ferramentas => {
                // Tabela somente na aba contextual; não mais em Ferramentas.
                grupos.push(self.faixa_grupo("Automação", self.faixa_catalogo_funcoes()));
                grupos.push(self.faixa_grupo("Aba", self.faixa_pilha(vec![
                    self.faixa_pequeno("Renomear aba…", "edit", Some(Msg::EditarNomeAba), false),
                    self.faixa_pequeno("Duplicar aba", "copy", Some(Msg::DuplicarAba), false),
                    self.faixa_pequeno("Excluir aba", "trash", (self.workbook.len()>1).then_some(Msg::RemoverAba), false),
                ])));
                grupos.push(self.faixa_grupo("Proteção", self.faixa_pilha(vec![
                    self.faixa_pequeno("Proteger intervalo", "lock", None, false),
                    self.faixa_pequeno("Intervalos protegidos", "lock", None, false),
                ])));
                grupos.push(self.faixa_grupo("Aplicativo", self.faixa_linha(vec![
                    self.faixa_grande("Ajuda", "guia_funcoes", Some(Msg::Ajuda), false),
                    self.faixa_grande("Configurações", "settings", Some(Msg::Configuracoes), false),
                    self.faixa_pilha(vec![
                        self.faixa_pequeno("Monitor", "device-desktop", Some(Msg::Monitor), false),
                        self.faixa_pequeno("Atalhos", "atalhos_teclado", Some(Msg::Ajuda), false),
                        self.faixa_pequeno("Sobre", "informacao", Some(Msg::MostrarSobre), false),
                    ]),
                ])));
            }
        }
        grupos
    }

    pub(super) fn view_faixa(&self) -> Element<'_, Msg> {
        let todos = self.montar_grupos_faixa();
        let limite = self.faixa_visiveis().min(todos.len());
        let possui_extras = limite < todos.len();
        let mut grupos:Vec<Element<'_,Msg>>=Vec::new();
        if self.faixa_ativa==AbaFaixa::Inicio {
            let plano=faixa_responsiva::planejar(self.tamanho_principal.width);
            for (indice,inteiro) in todos.into_iter().take(limite).enumerate() {
                grupos.push(match plano.niveis[indice] {
                    Nivel::Completo=>inteiro,
                    Nivel::Compacto=>self.faixa_inicio_compacto(indice),
                    Nivel::Recolhido=>self.faixa_inicio_recolhido(indice),
                });
            }
        } else {
            grupos.extend(todos.into_iter().take(limite));
        }
        if possui_extras {
            let botao_mais = button(column![
                feather_ui::icones::icone("chevron-down", 19.0),
                text("Mais").size(11),
            ].spacing(4).align_x(Alignment::Center))
                .padding([7, 12])
                .style(estilo_botao_grande(self.faixa_extras_aberto))
                .on_press(Msg::AlternarMenuExtras);
            grupos.push(ui::grupo_faixa_compacto(botao_mais.into(), "Ferramentas"));
        }
        // Sem widget Scrollable: nunca aparece uma barra horizontal na Ribbon.
        let conteudo = row(grupos)
            .spacing(2)
            .align_y(Alignment::Center)
            .height(Length::Fixed(ALTURA_CONTROLES_FAIXA));
        container(conteudo)
            .height(Length::Fixed(ALTURA_FAIXA_NATIVA))
            .padding([2, 5])
            .width(Length::Fill).into()
    }

    /// Menu de excedentes: mesmo conteúdo funcional dos grupos originais.
    /// Apenas a lista vertical interna pode rolar; nunca há scroll horizontal.
    pub(super) fn view_faixa_extras(&self) -> Element<'_, Msg> {
        let quantidade_visivel = self.faixa_visiveis();
        let grupos = self.montar_grupos_faixa();
        let ocultos = grupos.len().saturating_sub(quantidade_visivel);
        let mut lista = column![].spacing(5);
        for grupo in grupos.into_iter().skip(quantidade_visivel) {
            lista = lista.push(grupo);
        }
        let altura_disponivel = (self.tamanho_principal.height
            - ALTURA_FAIXA_NATIVA - 132.0).max(120.0).min(510.0);
        let painel = column![
            row![
                text(format!("Mais ferramentas · {ocultos} grupos")).size(13),
                Space::with_width(Length::Fill),
                button(feather_ui::icones::icone("x", 15.0))
                    .style(estilo_botao_pequeno(false))
                    .on_press(Msg::FecharMenuExtras),
            ].spacing(8).align_y(Alignment::Center),
            scrollable(lista)
                .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new()))
                .height(Length::Fixed(altura_disponivel)),
        ].spacing(8).padding(10);
        container(painel)
            .width(Length::Fixed(
                (self.tamanho_principal.width - 24.0).min(640.0).max(1.0)
            ))
            .style(estilo_popup_menu)
            .into()
    }

    /// Overlay abaixo da faixa, ancorado à direita. Não empurra a grade.
    pub(super) fn sobrepor_faixa_extras<'a>(
        &'a self, base: Element<'a, Msg>,
    ) -> Element<'a, Msg> {
        if !self.faixa_extras_aberto
            || self.faixa_visiveis() >= self.faixa_larguras().len()
        { return base; }
        let largura = (self.tamanho_principal.width - 24.0).min(640.0).max(1.0);
        let x = (self.tamanho_principal.width - largura - 12.0).max(8.0);
        let altura = (self.tamanho_principal.height
            - ALTURA_FAIXA_NATIVA - 132.0).max(120.0).min(510.0) + 72.0;
        let y_ideal = 43.0 + 52.0 + ALTURA_FAIXA_NATIVA;
        let y = y_ideal.min((self.tamanho_principal.height - altura - 8.0).max(8.0));
        let fora = iced::widget::mouse_area(
            Space::new(Length::Fill, Length::Fill)
        ).on_press(Msg::FecharMenuExtras);
        stack![
            base, fora,
            column![
                Space::with_height(Length::Fixed(y)),
                row![
                    Space::with_width(Length::Fixed(x)),
                    self.view_faixa_extras(),
                ],
            ],
        ].height(Length::Fill).into()
    }
}

#[cfg(test)]
mod testes_acoes_legadas {
    use super::*;

    #[test]
    fn ribbon_comandos_typescript_tem_tres_abas_nativas() {
        let mut app = App::default();
        for faixa in [AbaFaixa::Inicio, AbaFaixa::Dados, AbaFaixa::Exibir] {
            app.faixa_ativa = faixa;
            assert!(!app.montar_grupos_faixa().is_empty());
        }
    }

    #[test]
    fn modo_tela_inteira_alterna_sem_modificar_a_planilha() {
        let mut app = App::default();
        let versao = app.workbook.aba(0).unwrap().sheet.len();
        assert!(!app.tela_inteira);
        let _ = app.update(Msg::TelaInteira);
        assert!(app.tela_inteira);
        let _ = app.update(Msg::TelaInteira);
        assert!(!app.tela_inteira);
        assert_eq!(app.workbook.aba(0).unwrap().sheet.len(), versao);
        assert!(!app.alterado);
    }

    #[test]
    fn estatisticas_da_ribbon_reutilizam_perfil_nativo() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 0, 0, "10".into());
        app.workbook.escrever_celula(0, 1, 0, "20".into());
        let _ = app.update(Msg::PerfilColunaAbrir);
        let visao = app.perfil_coluna.as_ref().expect("painel de perfil nativo");
        assert_eq!(visao.dados.numeros, 2);
        assert_eq!(visao.dados.linhas, 2);
        assert!(!app.alterado);
    }
}

/// Comando ainda sem motor: fica desabilitado e a dica diz o que fará.
fn em_breve<'a>(botao: Element<'a, Msg>, dica: &'static str) -> Element<'a, Msg> {
    iced::widget::tooltip(
        botao,
        container(text(dica).size(11)).padding([4, 8]).style(estilo_dica),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}
